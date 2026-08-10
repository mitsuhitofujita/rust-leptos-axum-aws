//! Generates the action-type icon catalog from the pinned `lucide-leptos`.
//!
//! DR-0014 chose Lucide for the icons and `lucide-leptos` as the way to render
//! them, and named generating our own representation as the fallback "if the
//! crate's release cadence, its category-level feature granularity, or its
//! bundle cost becomes the binding constraint". The bundle cost did: the
//! crate's components carry five reactive props and a derived signal each, none
//! of which this application varies, and 725 copies of that came to +1.69 MB of
//! wasm against 258 KB of actual geometry.
//!
//! So this takes the geometry and leaves the components. It reads the crate's
//! own source out of the registry, keeps the icons [`CATEGORIES`] admits, and
//! writes two tables:
//!
//! - `crates/shared/src/icon_names.rs` — the names alone, so the API can reject
//!   one the picker could not have produced. It is compiled for a target Leptos
//!   never reaches (DR-0002), which is why it holds no markup.
//! - `crates/app/src/icon_catalog.rs` — the names, the official English names
//!   the picker searches, and the geometry each one draws.
//!
//! Run it with `just icons`. Nothing runs it automatically, so the generated
//! files and the pin agree only because that recipe was run after one of them
//! moved.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The Lucide categories the catalog is drawn from.
///
/// Category is the only granularity the crate's `cfg` gates offer, and Lucide's
/// categories are not shaped like this application: the eight icons in the
/// design reference alone span eight of them, so this is what admits a catalog
/// worth searching rather than a list of icons anyone chose.
const CATEGORIES: &[&str] = &[
    "animals",
    "buildings",
    "food-beverage",
    "home",
    "medical",
    "nature",
    "navigation",
    "people",
    "sports",
    "text",
    "time",
    "transportation",
    "travel",
    "weather",
];

const LUCIDE: &str = "lucide-leptos";
const LOCKFILE: &str = "Cargo.lock";
const NAMES_OUTPUT: &str = "crates/shared/src/icon_names.rs";
const CATALOG_OUTPUT: &str = "crates/app/src/icon_catalog.rs";

fn main() -> Result<(), String> {
    let root = repository_root();

    let lockfile = read(&root.join(LOCKFILE))?;
    let version = locked_version(&lockfile)
        .ok_or_else(|| format!("{LOCKFILE} names no {LUCIDE}; run `cargo fetch` first"))?;

    let source = lucide_source(&version)?;
    let icons = catalog(&source)?;
    if icons.is_empty() {
        return Err(format!(
            "no icon in {LUCIDE} {version} belongs to any of {CATEGORIES:?}"
        ));
    }

    fs::write(root.join(NAMES_OUTPUT), names_table(&icons, &version))
        .map_err(|err| format!("could not write {NAMES_OUTPUT}: {err}"))?;
    fs::write(root.join(CATALOG_OUTPUT), catalog_table(&icons, &version))
        .map_err(|err| format!("could not write {CATALOG_OUTPUT}: {err}"))?;

    let markup: usize = icons.iter().map(|icon| icon.geometry.len()).sum();
    println!(
        "{} icons from {LUCIDE} {version} in {} categories, {markup} bytes of geometry",
        icons.len(),
        CATEGORIES.len(),
    );
    Ok(())
}

/// One entry, in the three shapes the generated files need it in.
struct Icon {
    /// The canonical kebab-case Lucide name. This is what is stored.
    name: String,
    /// The official English name, as the picker shows and searches it.
    display: String,
    /// The children of the icon's `<svg>`, verbatim.
    geometry: String,
}

/// Every icon in the crate that at least one enabled category admits.
///
/// Sorted and deduplicated by name: both outputs are searched by binary search,
/// and the crate declares each icon twice — once as a `mod` and once as a
/// `pub use` — where the second sighting is not a second icon.
fn catalog(source: &Path) -> Result<Vec<Icon>, String> {
    let mut icons = BTreeMap::new();

    for (module, features) in modules(&read(&source.join("lib.rs"))?) {
        if !features
            .iter()
            .any(|feature| CATEGORIES.contains(&feature.as_str()))
        {
            continue;
        }

        let file = source.join(format!("{module}.rs"));
        let geometry = geometry(&read(&file)?)
            .ok_or_else(|| format!("no <svg> children in {}", file.display()))?;

        let name = kebab_case(&module);
        let display = name
            .split('-')
            .map(capitalized)
            .collect::<Vec<_>>()
            .join(" ");
        icons.insert(
            name.clone(),
            Icon {
                name,
                display,
                geometry,
            },
        );
    }

    Ok(icons.into_values().collect())
}

/// Every `mod` in the crate's `lib.rs` with the features gating it.
///
/// The gates are what the categories select on, and they are not one per icon:
/// an icon appears under every category Lucide filed it under, and the crate
/// turns that into `#[cfg(any(feature = "…", …))]` — sometimes over several
/// lines, which is why this accumulates until the parentheses close rather than
/// reading a line at a time.
fn modules(lib: &str) -> Vec<(String, Vec<String>)> {
    let mut modules = Vec::new();
    let mut attribute = String::new();
    let mut depth = 0isize;

    for line in lib.lines().map(str::trim) {
        if depth > 0 {
            attribute.push(' ');
            attribute.push_str(line);
            depth += parenthesis_depth(line);
            continue;
        }

        if line.is_empty() {
            continue;
        }

        if line.starts_with("#[cfg(") {
            attribute.clear();
            attribute.push_str(line);
            depth = parenthesis_depth(line);
            continue;
        }

        if let Some(rest) = line.strip_prefix("mod ") {
            // `mod r#box;` — a handful of icons are named after keywords. The
            // file on disk is `box.rs`, so the escape is not part of the name.
            let module = rest
                .trim_end_matches(';')
                .trim()
                .trim_start_matches("r#")
                .to_owned();
            modules.push((module, features(&attribute)));
        }

        // Anything else ends whatever the attribute above was attached to,
        // including the `pub use` half of the file.
        attribute.clear();
    }

    modules
}

/// The children of an icon module's `<svg>`, with the wrapper left behind.
///
/// The wrapper is the same for every icon and is written once, in
/// `crates/app/src/icons.rs`. What differs is only what is inside it — and what
/// is inside it is one element per line in every module of the pinned crate.
fn geometry(module: &str) -> Option<String> {
    let mut lines = module.lines().map(str::trim);

    // The opening `<svg` spans many lines and ends on one holding just `>`.
    lines.by_ref().find(|line| *line == ">")?;

    let mut markup = String::new();
    for line in lines {
        if line == "</svg>" {
            return Some(markup);
        }
        markup.push_str(line);
    }

    None
}

/// The canonical Lucide name behind a module name.
///
/// Underscores become hyphens, with two exceptions. Lucide writes `3d` and
/// `2x2` as single tokens, and the crate's module names split them at every
/// boundary, so `axis_3_d` and `grid_2_x_2` would otherwise come back as
/// `axis-3-d` and `grid-2-x-2` — names Lucide does not have, and therefore names
/// no stored value may be.
fn kebab_case(module: &str) -> String {
    let segments: Vec<&str> = module.split('_').collect();
    let mut name: Vec<String> = Vec::with_capacity(segments.len());
    let mut index = 0;

    while index < segments.len() {
        let segment = segments[index];
        if is_number(segment) {
            if let (Some(&"x"), Some(&next)) = (segments.get(index + 1), segments.get(index + 2))
                && is_number(next)
            {
                name.push(format!("{segment}x{next}"));
                index += 3;
                continue;
            }
            if segments.get(index + 1) == Some(&"d") {
                name.push(format!("{segment}d"));
                index += 2;
                continue;
            }
        }

        name.push(segment.to_owned());
        index += 1;
    }

    name.join("-")
}

/// The version of [`LUCIDE`] the lockfile resolves to.
///
/// The lockfile rather than the manifest, because the manifest's `3.26.0` is a
/// requirement and the registry holds directories named after exact versions.
fn locked_version(lockfile: &str) -> Option<String> {
    let package = lockfile.find(&format!("name = \"{LUCIDE}\""))?;
    let version = lockfile[package..].find("version = ")? + package;
    quoted(&lockfile[version..]).into_iter().next()
}

/// The unpacked crate's `src`, wherever cargo put it.
///
/// `crates/icongen` depends on the crate for exactly this: the dependency is
/// what pins the version in the lockfile and what makes cargo unpack the source
/// here. No feature is enabled, so nothing of it is compiled and nothing of it
/// ships — the geometry travels as the generated text instead.
fn lucide_source(version: &str) -> Result<PathBuf, String> {
    let cargo_home = std::env::var("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .map_err(|_| "neither CARGO_HOME nor HOME is set".to_owned())?;

    let registry = cargo_home.join("registry/src");
    let sources =
        fs::read_dir(&registry).map_err(|err| format!("no unpacked registry sources: {err}"))?;

    // The registry directory carries a hash of the source it came from, and
    // there can be more than one.
    for source in sources.flatten() {
        let candidate = source
            .path()
            .join(format!("{LUCIDE}-{version}"))
            .join("src");
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "{LUCIDE} {version} is not unpacked under {}; run `cargo fetch` first",
        registry.display()
    ))
}

fn names_table(icons: &[Icon], version: &str) -> String {
    let mut out = header(icons.len(), version);
    out.push_str(
        "\n\
         /// Every icon an action type may name, in the order [`is_known`] searches.\n\
         pub const ICON_NAMES: &[&str] = &[\n",
    );
    for icon in icons {
        out.push_str(&format!("    \"{}\",\n", icon.name));
    }
    out.push_str(
        "];\n\
         \n\
         /// Whether this is a name the picker could have produced.\n\
         ///\n\
         /// The API checks it before storing one. The picker is the only control\n\
         /// surface that offers a name, but it is not the only way a request can\n\
         /// arrive — DR-0014.\n\
         pub fn is_known(name: &str) -> bool {\n\
         \x20   ICON_NAMES.binary_search(&name).is_ok()\n\
         }\n",
    );
    out
}

fn catalog_table(icons: &[Icon], version: &str) -> String {
    let mut out = header(icons.len(), version);
    out.push_str(
        "\n\
         /// One icon: what is stored, what the picker shows and searches, and what\n\
         /// draws it.\n\
         pub struct Icon {\n\
         \x20   /// The canonical kebab-case Lucide name — what an action type stores.\n\
         \x20   pub name: &'static str,\n\
         \x20   /// The official English name, which the picker shows and filters on.\n\
         \x20   pub display: &'static str,\n\
         \x20   /// The children of the icon's `<svg>`. The wrapper around them is the\n\
         \x20   /// same for every icon and is written once, in `crate::icons::Glyph`.\n\
         \x20   pub geometry: &'static str,\n\
         }\n\
         \n\
         /// The supported catalog, ordered by [`Icon::name`].\n\
         pub static CATALOG: &[Icon] = &[\n",
    );
    for icon in icons {
        out.push_str(&format!(
            "    Icon {{\n\
             \x20       name: \"{}\",\n\
             \x20       display: \"{}\",\n\
             \x20       geometry: r#\"{}\"#,\n\
             \x20   }},\n",
            icon.name, icon.display, icon.geometry
        ));
    }
    out.push_str(
        "];\n\
         \n\
         /// The catalog entry for a stored name, if it is still one of ours.\n\
         ///\n\
         /// A name this does not know is not an error here: the identifier comes\n\
         /// over the wire and the catalog is this build's, so the two can disagree\n\
         /// the moment an action type outlives the category that admitted its icon\n\
         /// — DR-0014.\n\
         pub fn find(name: &str) -> Option<&'static Icon> {\n\
         \x20   CATALOG\n\
         \x20       .binary_search_by(|icon| icon.name.cmp(name))\n\
         \x20       .ok()\n\
         \x20       .map(|index| &CATALOG[index])\n\
         }\n",
    );
    out
}

fn header(count: usize, version: &str) -> String {
    format!(
        "//! The supported action-type icon catalog: {count} icons from\n\
         //! `{LUCIDE}` {version}, in the categories `crates/icongen` selects.\n\
         //!\n\
         //! Generated by `just icons`. Do not edit — change `crates/icongen` or the\n\
         //! pin it reads, and run that again.\n"
    )
}

/// Every `feature = "…"` named in a `cfg` attribute.
fn features(attribute: &str) -> Vec<String> {
    let mut features = Vec::new();
    for part in attribute.split("feature = ").skip(1) {
        features.extend(quoted(part).into_iter().next());
    }
    features
}

/// The double-quoted strings in a fragment, in order.
fn quoted(fragment: &str) -> Vec<String> {
    fragment
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

fn parenthesis_depth(line: &str) -> isize {
    line.chars()
        .map(|character| match character {
            '(' => 1,
            ')' => -1,
            _ => 0,
        })
        .sum()
}

fn is_number(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit())
}

fn capitalized(segment: &str) -> String {
    let mut characters = segment.chars();
    match characters.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + characters.as_str(),
        None => String::new(),
    }
}

/// The workspace root, relative to this crate rather than to the caller's
/// working directory, so `cargo run -p icongen` works from anywhere.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("could not read {}: {err}", path.display()))
}
