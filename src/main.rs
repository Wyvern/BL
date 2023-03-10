use std::*;

mod util;
use util::*;

fn main() {
    let arg = env::args().nth(1).unwrap_or_else(|| {
        println!("Please input URL argument..");
        process::exit(0);
    });

    let mut next = parse(&arg);

    while !next.is_empty() {
        //dbg!(&next);
        next = parse(&next);
    }
}

///Get scheme and host info from valid url string
fn scheme_host(addr: &str) -> [String; 2] {
    let split = addr.split_once("://").unwrap_or_else(|| {
        println!("Invalid URL address.");
        process::exit(0);
    });
    let scheme = split.0;
    if scheme.is_empty() || {
        let lo = scheme.to_lowercase();
        !(lo == "http" || lo == "https")
    } {
        println!("Invalid http(s) protocol.");
        process::exit(0);
    }
    let rest = split.1;
    let host = &rest[..rest.find('/').unwrap_or(rest.len())];
    if host.is_empty() {
        println!("Invalid host info.");
        process::exit(0);
    }

    [scheme.to_owned(), host.to_owned()]
}

///Check host info and Generate img/src/next selector data
fn check_host(host: &str) -> [String; 4] {
    let data = &website();
    let site = data
        .members()
        .find(|&s| {
            s["Site"]
                .as_str()
                .unwrap()
                .split_terminator(',')
                .any(|s| s == host.trim_start_matches("www."))
        })
        .unwrap_or_else(|| {
            println!("Unsupported website.. 🌏[{host}]💥");
            process::exit(0);
        });
    let next = site["Next"].as_str().unwrap_or("");
    let album = site["Album"].as_str().unwrap_or("");
    [
        site["Img"].as_str().unwrap().to_owned(),
        site["Src"].as_str().unwrap().to_owned(),
        next.to_string(),
        album.to_owned(),
    ]
}

///Fetch web page generate html content
fn get_html(addr: &str) -> String {
    let [_, host] = scheme_host(addr);
    check_host(&host);
    let out = process::Command::new("curl")
        .args([
            addr,
            "-e",
            host.as_str(),
            "-A",
            "Mozilla Firefox",
            "-s",
            "-L",
        ])
        .output()
        .unwrap_or_else(|e| {
            println!("{e}");
            process::exit(0);
        });
    let res = String::from_utf8_lossy(&out.stdout).to_string();
    if res.is_empty() {
        println!("Get html failed, please check url address.");
        process::exit(0);
    }
    res
}

///Parse photos in web url
fn parse(addr: &str) -> String {
    let [scheme, host] = scheme_host(addr);
    let [img, src, mut next, album] = check_host(&host);
    let html = get_html(addr);
    let page = crabquery::Document::from(html);
    let imgs = page.select(img.as_str());
    let titles = page.select("title");
    let title = titles
        .first()
        .unwrap_or_else(|| {
            println!("Not a valid html page.");
            process::exit(0);
        })
        .text()
        .expect("NO title text.");
    let mut t = title.trim();
    if t.contains(['-', '_', '|', '–']) {
        t = t[..t.rfind(['-', '_', '|', '–']).unwrap()].trim();
    }
    let albums = if album.is_empty() {
        vec![]
    } else {
        page.select(album.as_str())
    };
    let hasAlbum = !album.is_empty() && !albums.is_empty();
    match (hasAlbum, !imgs.is_empty()) {
        (true, true) => println!(
            "Totally found {} 🗺️  and {} 🏞️  in 📝: {} ",
            albums.len(),
            imgs.len(),
            t
        ),
        (true, false) => println!("Totally found {} 🗺️  in 📝: {} ", albums.len(), t),
        (false, true) => println!("Totally found {} 🏞️  in 📝: {} ", imgs.len(), t),
        (false, false) => {
            println!("∅ 🏞️  found in 📝: {t}");
            process::exit(0);
        }
    }

    if t.to_lowercase().contains("page") {
        t = t[..t.to_lowercase().rfind("page").unwrap()]
            .trim()
            .trim_end_matches(['-', '_', '|', '–'])
            .trim();
    };
    if t.contains(['(', ',', '集']) {
        t = t[..t.rfind(['(', ',', '集']).unwrap()].trim();
    };
    let canonicalize_url = |u: &str| {
        if !u.starts_with("http") {
            if u.starts_with("//") {
                format!("{scheme}:{u}")
            } else if u.starts_with('/') {
                format!("{scheme}://{host}{u}")
            } else {
                format!("{}/{u}", &addr[..addr.rfind('/').unwrap()])
            }
        } else {
            u.to_owned()
        }
    };
    match (hasAlbum, !imgs.is_empty()) {
        (_, true) => {
            for img in imgs {
                let src = img.attr(src.as_str()).expect("Invalid img[src] selector!");
                let mut src = src.as_str();
                src = &src[src.rfind("?url=").map(|p| p + 5).unwrap_or(0)..];
                src = &src[..src.rfind('?').unwrap_or(src.len())];
                let file = canonicalize_url(src);
                download(t, &file);
            }
        }
        (true, false) => {
            let mut all = false;

            for (i, alb) in albums.iter().enumerate() {
                let mut parseAlbum = || {
                    let mut href = alb.attr("href").unwrap_or_else(|| {
                        alb.parent()
                            .unwrap()
                            .attr("href")
                            .expect("NO a[@href] attr found.")
                    });
                    let album_url = canonicalize_url(&href);
                    next = parse(&album_url);
                    while !next.is_empty() {
                        next = parse(&next);
                    }
                };

                if all {
                    parseAlbum();
                } else {
                    use io::*;
                    let mut stdin = io::stdin();
                    let mut stdout = io::stdout();
                    let mut t = alb.text().expect("NO Album title found.");
                    writeln!(
                        stdout,
                        "Do you want to download Album <{}/{}>: {}?",
                        i + 1,
                        albums.len(),
                        t.trim()
                    );
                    write!(stdout, "[Yes⏎/No/All/Cancel]: ");
                    stdout.flush();

                    let mut input = String::new();
                    stdin.read_line(&mut input).unwrap_or_else(|e| {
                        println!("{e}");
                        process::exit(0);
                    });
                    input.make_ascii_lowercase();

                    match input.trim() {
                        "y" | "yes" | "" => parseAlbum(),
                        "n" | "no" => continue,
                        "a" | "all" => {
                            all = true;
                            parseAlbum()
                        }
                        _ => {
                            println!("Cancel all albums download.");
                            break;
                        }
                    }
                }
            }
        }
        (false, false) => (),
    }

    if (!next.is_empty()) {
        let nexts = page.select(&next);
        check_next(nexts, addr)
    } else {
        String::default()
    }
}

///Perform photo download operation
fn download(dir: &str, src: &str) {
    #[cfg(all(feature = "download", any(not(test), feature = "batch")))]
    {
        let dir = path::Path::new(dir);
        if (!dir.exists()) {
            fs::create_dir(dir).unwrap_or_else(|e| {
                println!("{e}");
                process::exit(0);
            });
        }

        let name = src[src.rfind('/').unwrap() + 1..].trim_start_matches(['-', '_']);
        let host = &src[..src[10..].find('/').unwrap_or(src.len() - 10) + 10];
        let wget = format!("wget {src} -O {name} --referer={host} -U \"Mozilla Firefox\" -q");
        let curl = format!("curl {src} -o {name} -e {host} -A \"Mozilla Firefox\" -L -s");
        //dbg!(&curl);
        if (dir.exists() && !dir.join(name).exists()) {
            #[cfg(feature = "curl")]
            process::Command::new("curl")
                .current_dir(dir)
                .args([
                    src,
                    "-o",
                    name,
                    "-e",
                    host,
                    "-A",
                    "Mozilla Firefox",
                    "-L",
                    //"--location-trusted",
                    "-s",
                ])
                .spawn();

            #[cfg(feature = "wget")]
            process::Command::new("wget")
                .current_dir(dir)
                .args([
                    &src,
                    format!("--referer={host}").as_str(),
                    "-U",
                    "Mozilla Firefox",
                    "-q",
                ])
                .spawn();
        }
    }
}

///Check next page info
fn check_next(nexts: Vec<crabquery::Element>, cur: &str) -> String {
    let mut next: String;
    if nexts.is_empty() {
        next = String::default();
        //println!("NO next page <element> found.")
    } else if nexts.len() == 1 {
        let element = &nexts[0];
        if element.tag().unwrap() == "span" {
            let items = element.parent().unwrap().children();
            let mut tags = items.rsplit(|e| e.tag().unwrap() == "span");
            let a = tags.next().unwrap();
            if a.is_empty() {
                next = String::default();
            } else {
                next = a[0].attr("href").unwrap();
            }
        } else {
            next = nexts[0].attr("href").unwrap();
        }
    } else {
        let element = &nexts[0];
        if element.tag().unwrap() == "div" && nexts.len() == 2 {
            let tags = element.children();
            let mut rest = tags.split(|tag| {
                if tag.children().is_empty() {
                    tag.tag().unwrap() == "span"
                } else {
                    tag.children()[0]
                        .attr("class")
                        .unwrap()
                        .contains("is-current")
                }
            });
            let s = rest.next_back().unwrap();
            if s.is_empty() {
                next = String::default()
            } else if s[0].children().is_empty() {
                next = s[0].attr("href").unwrap()
            } else {
                next = s[0].children()[0].attr("href").unwrap()
            }
        } else {
            let item = nexts[nexts.len() - 2..].iter().rfind(|&n| {
                let mut t = n.text();
                if t.is_some() && t.clone().unwrap().is_empty() {
                    t.take();
                }
                match t {
                    Some(text) => {
                        let l = text.to_lowercase();
                        l.contains('下') || l.contains("next") || (n.attr("target").is_some())
                    }
                    None => {
                        t = n.attr("title");
                        match t {
                            Some(title) => {
                                let l = title.to_lowercase();
                                l.contains('下') || l.contains("next")
                            }
                            None => {
                                let span = n.select("span.currenttext");
                                t = span[0].text();
                                match t {
                                    Some(text) => {
                                        let l = text.to_lowercase();
                                        l.contains('下') || l.contains("next")
                                    }
                                    None => false,
                                }
                            }
                        }
                    }
                }
            });
            next = match item {
                Some(v) => v
                    .attr("href")
                    .expect("NO [href] attr found in <next> link."),
                None => String::default(),
            };
        }
    }
    // if !next.is_empty() && !next[next.rfind('/').unwrap()..].contains(['_', '-', '?']) {
    //     next = String::default();
    // }
    if !next.is_empty() && !next.starts_with("http") {
        if next.trim() == "/" || next.trim() == "#" {
            next = String::default();
        } else {
            next = format!(
                "{}{}",
                if next.starts_with("//") {
                    &cur[..cur.find("//").unwrap()]
                } else if next.starts_with('/') {
                    &cur[..cur[10..].find('/').unwrap() + 10]
                } else {
                    &cur[..=cur.rfind('/').unwrap()]
                },
                next
            );
        }
    }

    test_dbg(next)
}

///WebSites Json config data
fn website() -> json::JsonValue {
    json::parse(include_str!("web.json")).unwrap_or_else(|e| {
        println!("{e}");
        process::exit(0);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html() {
        let addr = "https://hotgirl.biz/xiuren-no-6069-%e9%a1%be%e4%b9%94%e6%a5%a0/";
        let res = get_html(addr);
        dbg!(&res);
    }

    // https://girldreamy.com/

    #[test]
    fn try_it() {
        let addr = "https://bestgirlsexy.com/wp-content/uploads/2023/03/";
        parse(addr);
    }

    #[test]
    fn htmlq() {
        let addr = "https://bestgirlsexy.com/ligui%e4%b8%bd%e6%9f%9c-2023-01-25-xin-xin-and-shio-shio-and-liang-liang/";
        let [_, host] = scheme_host(addr);
        let [img, src, mut next, album] = check_host(&host);
        let html = get_html(addr);
        use process::*;
        let mut cmd = Command::new("htmlq")
            .args([img])
            .stdin(Stdio::piped())
            //.stdout(Stdio::piped())
            .spawn()
            .expect("Execute htmlq failed.");
        let mut stdin = cmd.stdin.as_ref().expect("Failed to open stdin.");
        use io::*;
        stdin.write_all(html.as_bytes()).expect("Failed to write.");
        cmd.wait_with_output().expect("Failed to get piped stdout.");
    }

    #[test]
    fn run() {
        let mut addr = "https://girldreamy.com/category/china/xiuren/page/30";
        let page = &addr[addr.rfind('/').unwrap() + 1..];
        let num = page.parse::<u16>().expect("Parse page number failed.");
        (0_u16..=4).map(|i| num - i).for_each(|p| {
            let mut idx = format!("{}{p}", &addr[..=addr.rfind('/').unwrap()]);
            test_dbg(&idx);
            idx = parse(&idx);
            while !idx.is_empty() {
                idx = parse(&idx);
            }
        });
    }

    #[test]
    fn pause() {
        use io::*;
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();

        write!(stdout, "Press any key to continue...");
        stdout.flush();

        // Read a single byte and discard
        //let _ = stdin.read(&mut []).unwrap();
        stdin.read(&mut []);
    }
}
