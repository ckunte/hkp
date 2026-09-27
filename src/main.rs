use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write as IoWrite};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

use lopdf::{dictionary, Document, Object, ObjectId};
use rayon::prelude::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

fn print_help() {
    print!(
r#"Usage: hk <command> [argument]

  cmbi          Combine PDFs in current folder → <dirname>-comb.pdf
  cmbs          Combine PDFs per subdirectory → <subdir>-comb.pdf
  cpng          Compress PNGs (q60-80) into ./compressed/ [pngquant]
  cr            Squash git history to one commit, force-push [git]
  ffp           Set permissions: dirs 755, files 644
  sffn          Sanitise names: spaces/dots→underscores, strip hyphens
  srv [port]    Serve current directory over HTTP (default port 8000)
  help          Show this help

  pngquant: brew install pngquant · apt/dnf install pngquant
            winget/scoop install pngquant · https://pngquant.org

AUTHOR
    Written by Chetan Kunté and Claude
    https://ckunte.net
"#
    );
}

// ---------------------------------------------------------------------------
// Parallel filesystem walk
//
// Each directory's entries are read serially, but subdirectory recursion is
// dispatched to rayon's thread pool so all cores are used.  file_type() reads
// d_type from the readdir result — zero extra stat/lstat syscalls.
// ---------------------------------------------------------------------------

fn walk(root: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    // Pre-size to avoid repeated reallocations across a large tree.
    let dirs:  Mutex<Vec<(usize, PathBuf)>> = Mutex::new(Vec::with_capacity(512));
    let files: Mutex<Vec<PathBuf>>           = Mutex::new(Vec::with_capacity(4096));
    walk_inner(root, 0, &dirs, &files);
    let mut d = dirs.into_inner().unwrap();
    // Deepest dirs first so bottom-up rename is safe.
    // Sort by the depth integer we tracked during the walk — O(N log N) with
    // cheap integer comparisons instead of O(N log N × path_len) via
    // components().count() on every comparison.
    d.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    let dirs = d.into_iter().map(|(_, p)| p).collect();
    (dirs, files.into_inner().unwrap())
}

fn walk_inner(
    dir: &Path,
    depth: usize,
    dirs:  &Mutex<Vec<(usize, PathBuf)>>,
    files: &Mutex<Vec<PathBuf>>,
) {
    // Iterate read_dir directly — avoids allocating a Vec<DirEntry> per
    // directory just to traverse it once.
    let Ok(rd) = fs::read_dir(dir) else { return };

    let mut local_files = Vec::new();
    let mut subdirs     = Vec::new();

    for entry in rd.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() { continue; }
        let path = entry.path();
        if ft.is_dir() {
            subdirs.push(path);
        } else if ft.is_file() {
            local_files.push(path);
        }
    }

    // One lock per directory, not one lock per file.
    files.lock().unwrap().extend(local_files);

    // Recurse into subdirectories in parallel across all cores.
    subdirs.par_iter().for_each(|sub| walk_inner(sub, depth + 1, dirs, files));

    // Record subdirs with their depth for the sort in walk().
    dirs.lock().unwrap().extend(subdirs.into_iter().map(|p| (depth + 1, p)));
}

fn collect_sorted_pdfs(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else { return vec![]; };
    let mut pdfs: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();
    pdfs.sort_unstable();
    pdfs
}

fn dir_stem(dir: &Path) -> String {
    dir.canonicalize()
        .ok()
        .as_deref()
        .unwrap_or(dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("combined")
        .to_owned()
}

// ---------------------------------------------------------------------------
// ffp — fix permissions: dirs 755, files 644 (parallel chmod)
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn cmd_ffp() -> Result<()> {
    let cwd = env::current_dir()?;
    let (dirs, files) = walk(&cwd);

    fs::set_permissions(&cwd, fs::Permissions::from_mode(0o755))?;

    dirs.par_iter().for_each(|d| {
        let _ = fs::set_permissions(d, fs::Permissions::from_mode(0o755));
    });
    files.par_iter().for_each(|f| {
        let _ = fs::set_permissions(f, fs::Permissions::from_mode(0o644));
    });

    println!(
        "Permissions set: {} folder{} (755), {} file{} (644).",
        dirs.len() + 1,
        if dirs.len() + 1 == 1 { "" } else { "s" },
        files.len(),
        if files.len() == 1 { "" } else { "s" }
    );
    Ok(())
}

#[cfg(not(unix))]
fn cmd_ffp() -> Result<()> {
    Err("ffp is not supported on this platform (Unix permission modes only)".into())
}

// ---------------------------------------------------------------------------
// sffn — sanitise folder and file names
//
// Rules:
//   • Replace spaces with underscores in all names
//   • In directory names: replace every dot with an underscore
//   • In .pdf and .docx file stems: replace every dot with an underscore
//   • Strip trailing hyphens before the extension (e.g. "foo-.pdf")
//
// File renames are parallelised; directory renames stay sequential
// (deepest-first) so parent paths are not invalidated mid-run.
// ---------------------------------------------------------------------------

fn sanitize_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            ' ' | '\t' => '_',
            '.'        => '_',
            _          => c,
        })
        .collect()
}

fn sanitize_file_stem(stem: &str) -> String {
    let s: String = stem
        .chars()
        .map(|c| match c {
            ' ' | '\t' => '_',
            '.'        => '_',
            _          => c,
        })
        .collect();
    s.trim_end_matches(['-', '_']).to_owned()
}

fn cmd_sffn() -> Result<()> {
    let cwd = env::current_dir()?;
    let (dirs, files) = walk(&cwd);
    let count = AtomicUsize::new(0);

    // Rename .pdf and .docx files in parallel (each rename is independent).
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    files.par_iter().for_each(|path| {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "pdf" && ext != "docx" { return; }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None    => return,
        };
        let old_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let new_name = format!("{}.{}", sanitize_file_stem(stem), ext);
        if new_name != old_name {
            let new_path = path.parent().unwrap().join(&new_name);
            match fs::rename(path, &new_path) {
                Ok(_)  => { count.fetch_add(1, Ordering::Relaxed); }
                Err(e) => { errors.lock().unwrap()
                                .push(format!("{}: {}", path.display(), e)); }
            }
        }
    });

    // Rename directories sequentially, deepest-first.
    for path in &dirs {
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None    => continue,
        };
        let new_name = sanitize_dir_name(name);
        if new_name != name {
            let new_path = path.parent().unwrap().join(&new_name);
            match fs::rename(path, &new_path) {
                Ok(_)  => { count.fetch_add(1, Ordering::Relaxed); }
                Err(e) => { errors.lock().unwrap()
                                .push(format!("{}: {}", path.display(), e)); }
            }
        }
    }

    let n = count.load(Ordering::Relaxed);
    println!("Sanitised {} name{}.", n, if n == 1 { "" } else { "s" });
    for e in errors.into_inner().unwrap() {
        eprintln!("warning: {}", e);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// PDF merge (cmbi / cmbs)
// PDF pages must be merged in document order, so this stays sequential.
// ---------------------------------------------------------------------------

fn type_of(obj: &Object) -> &str {
    obj.as_dict()
        .ok()
        .and_then(|d| d.get(b"Type").ok())
        .and_then(|t| t.as_name().ok())
        .and_then(|n| std::str::from_utf8(n).ok())
        .unwrap_or("")
}

fn merge_pdfs(paths: &[PathBuf]) -> Result<Document> {
    let mut max_id: u32 = 1;
    let mut ordered_pages: Vec<(ObjectId, Object)> = Vec::new();
    let mut resources: BTreeMap<ObjectId, Object> = BTreeMap::new();

    for path in paths {
        let mut doc = Document::load(path)
            .map_err(|e| format!("{}: {}", path.display(), e))?;
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let mut page_pairs: Vec<(u32, ObjectId)> =
            doc.get_pages().into_iter().collect();
        page_pairs.sort_by_key(|(n, _)| *n);
        for (_, page_id) in page_pairs {
            if let Some(obj) = doc.objects.get(&page_id) {
                ordered_pages.push((page_id, obj.clone()));
            }
        }

        for (id, obj) in &doc.objects {
            match type_of(obj) {
                "Page" | "Pages" | "Catalog" => {}
                _ => { resources.insert(*id, obj.clone()); }
            }
        }
    }

    let mut merged = Document::new();
    merged.version = "1.5".to_owned();
    let pages_id:   ObjectId = (max_id, 0);
    let catalog_id: ObjectId = (max_id + 1, 0);

    for (id, obj) in resources { merged.objects.insert(id, obj); }

    let page_count = ordered_pages.len() as i64;
    let mut kids: Vec<Object> = Vec::with_capacity(ordered_pages.len());
    for (id, mut page) in ordered_pages {
        if let Ok(dict) = page.as_dict_mut() {
            dict.set("Parent", Object::Reference(pages_id));
        }
        merged.objects.insert(id, page);
        kids.push(Object::Reference(id));
    }

    merged.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type"  => "Pages",
            "Kids"  => Object::Array(kids),
            "Count" => page_count,
        }),
    );
    merged.objects.insert(
        catalog_id,
        Object::Dictionary(dictionary! {
            "Type"  => "Catalog",
            "Pages" => Object::Reference(pages_id),
        }),
    );
    merged.trailer.set("Root", Object::Reference(catalog_id));
    merged.trailer.set("Size", (merged.objects.len() + 1) as i64);
    merged.compress();

    Ok(merged)
}

fn combine_dir(dir: &Path) -> Result<()> {
    let pdfs = collect_sorted_pdfs(dir);
    if pdfs.is_empty() { return Ok(()); }
    let name   = dir_stem(dir);
    let output = dir.join(format!("{}-comb.pdf", name));
    print!(
        "  {} ({} file{}) → {} ... ",
        name,
        pdfs.len(),
        if pdfs.len() == 1 { "" } else { "s" },
        output.file_name().unwrap().to_string_lossy()
    );
    io::stdout().flush().ok();
    let mut doc = merge_pdfs(&pdfs)?;
    doc.save(&output)?;
    println!("done.");
    Ok(())
}

fn cmd_cmbi() -> Result<()> {
    let cwd  = env::current_dir()?;
    let pdfs = collect_sorted_pdfs(&cwd);
    if pdfs.is_empty() {
        println!("No PDF files found in the current directory.");
        return Ok(());
    }
    let name   = dir_stem(&cwd);
    let output = cwd.join(format!("{}-comb.pdf", name));
    print!(
        "Combining {} file{} → {} ... ",
        pdfs.len(),
        if pdfs.len() == 1 { "" } else { "s" },
        output.display()
    );
    io::stdout().flush().ok();
    let mut doc = merge_pdfs(&pdfs)?;
    doc.save(&output)?;
    println!("done.");
    Ok(())
}

fn cmd_cmbs() -> Result<()> {
    let cwd = env::current_dir()?;
    let mut dirs: Vec<PathBuf> = fs::read_dir(&cwd)?
        .flatten()
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    dirs.sort_unstable();

    if dirs.is_empty() {
        println!("No subdirectories found.");
        return Ok(());
    }

    println!("Combining PDFs in subdirectories of {}:", cwd.display());
    for dir in dirs {
        combine_dir(&dir)?;
    }
    println!("Done.");
    Ok(())
}

// ---------------------------------------------------------------------------
// cpng — compress PNG images (parallel pngquant invocations)
// ---------------------------------------------------------------------------

fn cmd_cpng() -> Result<()> {
    let cwd     = env::current_dir()?;
    let out_dir = cwd.join("compressed");
    fs::create_dir_all(&out_dir)?;

    let pngs: Vec<PathBuf> = fs::read_dir(&cwd)?
        .flatten()
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("png"))
                    .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    if pngs.is_empty() {
        println!("No PNG files found in the current directory.");
        return Ok(());
    }

    // Probe for pngquant before spawning parallel work.
    Command::new("pngquant")
        .arg("--version")
        .output()
        .map_err(|_| "pngquant not found — install it first (run `hk help`)")?;

    let count  = AtomicUsize::new(0);
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pngs.par_iter().for_each(|png| {
        let filename = png.file_name().unwrap();
        let output   = out_dir.join(filename);
        let result   = Command::new("pngquant")
            .args(["--quality=60-80", "--force", "--output"])
            .arg(&output)
            .arg(png)
            .output();
        match result {
            Ok(o) if o.status.success() => {
                count.fetch_add(1, Ordering::Relaxed);
            }
            Ok(_) => {
                errors.lock().unwrap()
                    .push(format!("{}: pngquant failed", png.display()));
            }
            Err(e) => {
                errors.lock().unwrap()
                    .push(format!("{}: {}", png.display(), e));
            }
        }
    });

    let n = count.load(Ordering::Relaxed);
    println!("Compressed {}/{} file{}. Output: {}",
        n, pngs.len(), if n == 1 { "" } else { "s" }, out_dir.display());
    for e in errors.into_inner().unwrap() {
        eprintln!("warning: {}", e);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// cr — recreate git repository (squash history)
// ---------------------------------------------------------------------------

fn cmd_cr() -> Result<()> {
    println!("Squashing git history …");
    let cmds: &[&[&str]] = &[
        &["git", "checkout", "--orphan", "newBranch"],
        &["git", "add", "-A"],
        &["git", "commit", "-m", "first commit"],
        &["git", "branch", "-D", "master"],
        &["git", "branch", "-m", "master"],
        &["git", "push", "-f", "origin", "master"],
        &["git", "gc", "--aggressive", "--prune=all"],
    ];
    for cmd in cmds {
        let status = Command::new(cmd[0])
            .args(&cmd[1..])
            .status()
            .map_err(|_| "git not found")?;
        if !status.success() {
            return Err(format!("git command failed: {}", cmd.join(" ")).into());
        }
    }
    println!("Done.");
    Ok(())
}

// ---------------------------------------------------------------------------
// srv — serve current folder over HTTP (one thread per connection)
// ---------------------------------------------------------------------------

fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut bytes  = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let h1 = bytes.next().and_then(|c| (c as char).to_digit(16));
            let h2 = bytes.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(h1), Some(h2)) = (h1, h2) {
                result.push((h1 * 16 + h2) as u8 as char);
            }
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css")                => "text/css",
        Some("js")                 => "application/javascript",
        Some("json")               => "application/json",
        Some("png")                => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif")                => "image/gif",
        Some("svg")                => "image/svg+xml",
        Some("pdf")                => "application/pdf",
        Some("txt") | Some("md")   => "text/plain; charset=utf-8",
        Some("woff")               => "font/woff",
        Some("woff2")              => "font/woff2",
        _                          => "application/octet-stream",
    }
}

fn serve_connection(mut stream: TcpStream, root: &Path) {
    let mut reader = BufReader::new(&stream);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() { return; }

    // Consume remaining headers.
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_)                        => break,
            Ok(_) if line == "\r\n" || line == "\n" => break,
            _ => {}
        }
    }

    // Parse path from "GET /path HTTP/1.x".
    let raw_path = request_line.split_whitespace().nth(1).unwrap_or("/").to_owned();
    let raw_path = raw_path.split('?').next().unwrap_or("/");
    let decoded  = url_decode(raw_path);
    let rel      = decoded.trim_start_matches('/');

    // Resolve path, guarding against directory traversal.
    let mut file_path = root.to_path_buf();
    for component in Path::new(rel).components() {
        use std::path::Component;
        if let Component::Normal(c) = component { file_path.push(c); }
    }

    if file_path.is_dir() { file_path.push("index.html"); }
    if !file_path.exists() {
        let with_html = file_path.with_extension("html");
        if with_html.exists() { file_path = with_html; }
    }

    let (status_line, body, content_type): (&str, Vec<u8>, &str) =
        if file_path.is_file() {
            match fs::read(&file_path) {
                Ok(data) => ("200 OK", data, mime_type(&file_path)),
                Err(_)   => ("500 Internal Server Error",
                              b"Internal Server Error".to_vec(), "text/plain"),
            }
        } else {
            ("404 Not Found", b"Not Found".to_vec(), "text/plain")
        };

    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status_line, content_type, body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&body);
}

fn cmd_srv(port: u16) -> Result<()> {
    let root     = env::current_dir()?;
    let listener = TcpListener::bind(("0.0.0.0", port))
        .map_err(|e| format!("Cannot bind to port {}: {}", port, e))?;
    println!("Serving {} on http://localhost:{} — Ctrl-C to stop", root.display(), port);

    let root = std::sync::Arc::new(root);
    for stream in listener.incoming() {
        if let Ok(s) = stream {
            let root = std::sync::Arc::clone(&root);
            std::thread::spawn(move || serve_connection(s, &root));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");

    let result: Result<()> = match cmd {
        "cmbi" => cmd_cmbi(),
        "cmbs" => cmd_cmbs(),
        "cpng" => cmd_cpng(),
        "cr"   => cmd_cr(),
        "ffp"  => cmd_ffp(),
        "sffn" => cmd_sffn(),
        "srv"  => {
            let port = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(8000);
            cmd_srv(port)
        }
        "help" | "--help" | "-h" => { print_help(); Ok(()) }
        other => {
            eprintln!("hk: unknown command '{}'\n", other);
            print_help();
            std::process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
