use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write as IoWrite};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use std::time::Duration;

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
  cr [--yes]    Squash git history to one commit, force-push [git]
  ffp           Set permissions: dirs 755, files 644
  sffn          Sanitise names: spaces/dots→underscores, strip hyphens
  srv [port] [--public]
                Serve current directory over HTTP (default 8000, localhost only)
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

/// Plan `from` -> `parent/new_name` only when it is safe: the new name must be
/// usable, must not already exist on disk, and must not be claimed by another
/// rename in this run.  `fs::rename` silently replaces an existing target on
/// Unix, so without this check "a b.pdf" and "a_b.pdf" would destroy one another.
fn plan_rename(
    from: &Path,
    new_name: &str,
    claimed: &mut HashSet<PathBuf>,
    skipped: &mut Vec<String>,
) -> Option<(PathBuf, PathBuf)> {
    let to = from.parent()?.join(new_name);
    if new_name.is_empty() || new_name.starts_with('.') {
        skipped.push(format!("{}: sanitised name '{}' is unusable", from.display(), new_name));
        return None;
    }
    if to.symlink_metadata().is_ok() || !claimed.insert(to.clone()) {
        skipped.push(format!("{}: target {} already exists", from.display(), to.display()));
        return None;
    }
    Some((from.to_path_buf(), to))
}

fn cmd_sffn() -> Result<()> {
    let cwd = env::current_dir()?;
    let (dirs, files) = walk(&cwd);
    let count = AtomicUsize::new(0);
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    let mut skipped: Vec<String> = Vec::new();

    // Plan every rename sequentially so collisions are detected
    // deterministically; only the (independent) execution is parallel.
    let mut file_plan: Vec<(PathBuf, PathBuf)> = Vec::new();
    for path in &files {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "pdf" && ext != "docx" { continue; }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let old_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let new_name = format!("{}.{}", sanitize_file_stem(stem), ext);
        if new_name != old_name {
            if let Some(p) = plan_rename(path, &new_name, &mut claimed, &mut skipped) {
                file_plan.push(p);
            }
        }
    }

    // Directories are renamed deepest-first so parent paths stay valid.
    let mut dir_plan: Vec<(PathBuf, PathBuf)> = Vec::new();
    for path in &dirs {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let new_name = sanitize_dir_name(name);
        if new_name != name {
            if let Some(p) = plan_rename(path, &new_name, &mut claimed, &mut skipped) {
                dir_plan.push(p);
            }
        }
    }

    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    file_plan.par_iter().for_each(|(from, to)| {
        match fs::rename(from, to) {
            Ok(_)  => { count.fetch_add(1, Ordering::Relaxed); }
            Err(e) => { errors.lock().unwrap()
                            .push(format!("{}: {}", from.display(), e)); }
        }
    });
    for (from, to) in &dir_plan {
        match fs::rename(from, to) {
            Ok(_)  => { count.fetch_add(1, Ordering::Relaxed); }
            Err(e) => { errors.lock().unwrap()
                            .push(format!("{}: {}", from.display(), e)); }
        }
    }

    let n = count.load(Ordering::Relaxed);
    println!("Sanitised {} name{}.", n, if n == 1 { "" } else { "s" });
    for e in skipped.into_iter().chain(errors.into_inner().unwrap()) {
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

/// Run git, return trimmed stdout on success.
fn git_out(args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|_| "git not found")?;
    if !out.status.success() {
        return Err(format!("git {} failed: {}", args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn git_run(args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .status()
        .map_err(|_| "git not found")?;
    if !status.success() {
        return Err(format!("git command failed: git {}", args.join(" ")).into());
    }
    Ok(())
}

fn cmd_cr(assume_yes: bool) -> Result<()> {
    if git_out(&["rev-parse", "--is-inside-work-tree"]).ok().as_deref() != Some("true") {
        return Err("not inside a git work tree".into());
    }
    let branch = git_out(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch == "HEAD" {
        return Err("detached HEAD — check out a branch first".into());
    }
    let remote = git_out(&["remote", "get-url", "origin"])
        .map_err(|_| "no 'origin' remote configured")?;
    // `git add -A` below would sweep up untracked files (possibly secrets),
    // so insist on a clean tree and let the user decide what goes in.
    if !git_out(&["status", "--porcelain"])?.is_empty() {
        return Err("working tree is not clean — commit, stash or ignore changes first".into());
    }

    println!("This will REPLACE ALL HISTORY of branch '{}' with a single commit", branch);
    println!("and force-push it to {}", remote);
    if !assume_yes {
        print!("Type the branch name to continue: ");
        io::stdout().flush().ok();
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if answer.trim() != branch {
            return Err("aborted".into());
        }
    }

    const TMP: &str = "hk-squash-tmp";
    println!("Squashing git history …");
    git_run(&["checkout", "--orphan", TMP])?;

    // Anything failing before the push leaves the original branch untouched;
    // put the user back on it.
    let prepare = git_run(&["add", "-A"])
        .and_then(|_| git_run(&["commit", "-m", "first commit"]))
        .and_then(|_| {
            let refspec = format!("{}:{}", TMP, branch);
            git_run(&["push", "--force-with-lease", "origin", &refspec])
        });
    if let Err(e) = prepare {
        let _ = git_run(&["checkout", "-f", &branch]);
        let _ = git_run(&["branch", "-D", TMP]);
        return Err(e);
    }

    git_run(&["branch", "-D", &branch])?;
    git_run(&["branch", "-m", &branch])?;
    git_run(&["branch", "--set-upstream-to", &format!("origin/{}", branch)])?;
    // Old commits stay reachable through the reflog until it expires, which
    // keeps a local recovery path (`git reflog`).
    git_run(&["gc", "--aggressive", "--prune=now"])?;
    println!("Done.");
    Ok(())
}

// ---------------------------------------------------------------------------
// srv — serve current folder over HTTP (one thread per connection)
// ---------------------------------------------------------------------------

const MAX_HEADER_BYTES: u64 = 16 * 1024;
const MAX_CONNECTIONS: usize = 64;
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Percent-decode into raw bytes, then validate as UTF-8.  Returns None for
/// malformed escapes or invalid UTF-8.
fn url_decode(s: &str) -> Option<String> {
    let mut out: Vec<u8> = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let h1 = bytes.next().and_then(|c| (c as char).to_digit(16))?;
                let h2 = bytes.next().and_then(|c| (c as char).to_digit(16))?;
                out.push((h1 * 16 + h2) as u8);
            }
            b'+' => out.push(b' '),
            _    => out.push(b),
        }
    }
    String::from_utf8(out).ok()
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

/// Map a request path to a file under `root` (which must be canonical).
/// Refuses `..`, hidden (dot-prefixed) components, and anything whose
/// canonical location — after resolving symlinks — lies outside `root`.
fn resolve_request(root: &Path, raw_path: &str) -> std::result::Result<PathBuf, u16> {
    use std::path::Component;
    let decoded = url_decode(raw_path).ok_or(400u16)?;
    if decoded.contains('\0') { return Err(400); }

    let mut file_path = root.to_path_buf();
    for component in Path::new(decoded.trim_start_matches('/')).components() {
        match component {
            Component::Normal(c) => {
                if c.to_string_lossy().starts_with('.') { return Err(404); }
                file_path.push(c);
            }
            Component::CurDir | Component::RootDir => {}
            Component::ParentDir | Component::Prefix(_) => return Err(400),
        }
    }

    if file_path.is_dir() { file_path.push("index.html"); }
    if !file_path.exists() {
        let with_html = file_path.with_extension("html");
        if with_html.exists() { file_path = with_html; }
    }

    let canonical = file_path.canonicalize().map_err(|_| 404u16)?;
    if !canonical.starts_with(root) || !canonical.is_file() { return Err(404); }
    Ok(canonical)
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _   => "Error",
    }
}

fn write_head(stream: &mut TcpStream, code: u16, content_type: &str, len: u64) -> io::Result<()> {
    let extra = if code == 405 { "Allow: GET, HEAD\r\n" } else { "" };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         X-Content-Type-Options: nosniff\r\n{}Connection: close\r\n\r\n",
        code, status_text(code), content_type, len, extra
    )
}

fn serve_connection(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    // Cap total header bytes so a never-ending line can't exhaust memory.
    let mut reader = BufReader::new((&stream).take(MAX_HEADER_BYTES));

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() { return; }

    let mut headers_done = false;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_)                          => break,
            Ok(_) if line == "\r\n" || line == "\n" => { headers_done = true; break; }
            _ => {}
        }
    }
    if !headers_done || !request_line.ends_with('\n') {
        let _ = write_head(&mut stream, 431, "text/plain", 0);
        return;
    }

    // "GET /path HTTP/1.x"
    let mut parts = request_line.split_whitespace();
    let method   = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("/");
    let raw_path = raw_path.split(['?', '#']).next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        let _ = write_head(&mut stream, 405, "text/plain", 0);
        return;
    }

    let path = match resolve_request(root, raw_path) {
        Ok(p)     => p,
        Err(code) => {
            let msg = status_text(code);
            let _ = write_head(&mut stream, code, "text/plain", msg.len() as u64);
            if method == "GET" { let _ = stream.write_all(msg.as_bytes()); }
            return;
        }
    };

    let file = match fs::File::open(&path) {
        Ok(f)  => f,
        Err(_) => { let _ = write_head(&mut stream, 500, "text/plain", 0); return; }
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    if write_head(&mut stream, 200, mime_type(&path), len).is_err() { return; }
    if method == "GET" {
        // Stream rather than loading the whole file into memory.
        let _ = io::copy(&mut file.take(len), &mut stream);
    }
}

/// Decrements the live-connection counter when a handler thread finishes.
struct ConnGuard(std::sync::Arc<AtomicUsize>);
impl Drop for ConnGuard {
    fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); }
}

fn cmd_srv(port: u16, public: bool) -> Result<()> {
    let root = env::current_dir()?.canonicalize()?;
    let host = if public { "0.0.0.0" } else { "127.0.0.1" };
    let listener = TcpListener::bind((host, port))
        .map_err(|e| format!("Cannot bind to {}:{}: {}", host, port, e))?;
    println!("Serving {} on http://localhost:{} — Ctrl-C to stop", root.display(), port);
    if public {
        eprintln!("warning: --public exposes this folder to every host that can reach this machine");
    }

    let root = std::sync::Arc::new(root);
    let live = std::sync::Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming().flatten() {
        // Over the cap: drop the connection instead of spawning another thread.
        if live.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            live.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let guard = ConnGuard(std::sync::Arc::clone(&live));
        let root  = std::sync::Arc::clone(&root);
        std::thread::spawn(move || {
            let _guard = guard;
            serve_connection(stream, &root);
        });
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
        "cr"   => cmd_cr(args[2..].iter().any(|a| a == "--yes" || a == "-y")),
        "ffp"  => cmd_ffp(),
        "sffn" => cmd_sffn(),
        "srv"  => {
            let public = args[2..].iter().any(|a| a == "--public");
            let port = args[2..].iter().find_map(|p| p.parse::<u16>().ok()).unwrap_or(8000);
            cmd_srv(port, public)
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
