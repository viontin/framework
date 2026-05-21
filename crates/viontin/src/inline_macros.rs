/// Embed an HTML file as `&'static str` at compile time.
///
/// # Examples
///
/// ```ignore
/// use viontin::html;
///
/// let page = html!("pages/index.html");
/// // -> contents of pages/index.html at compile time
/// ```
#[macro_export]
macro_rules! html {
    ($path:literal) => { include_str!($path) };
}

/// Embed a Markdown file as `&'static str` at compile time.
///
/// Rendered to HTML only if the `md` feature is enabled.
/// Otherwise returns the raw Markdown string.
///
/// # Examples
///
/// ```ignore
/// use viontin::md;
///
/// let doc = md!("docs/guide.md");
/// ```
#[macro_export]
macro_rules! md {
    ($path:literal) => { include_str!($path) };
}

/// Embed a JavaScript file as `&'static str` at compile time.
///
/// # Examples
///
/// ```ignore
/// use viontin::js;
///
/// let script = js!("assets/app.js");
/// ```
#[macro_export]
macro_rules! js {
    ($path:literal) => { include_str!($path) };
}

/// Embed a TypeScript file as `&'static str` at compile time.
///
/// # Examples
///
/// ```ignore
/// use viontin::ts;
///
/// let component = ts!("components/app.ts");
/// ```
#[macro_export]
macro_rules! ts {
    ($path:literal) => { include_str!($path) };
}

/// Define a domain with its allowed dependencies (requires `domain` feature).
///
/// Domains are the core building block of Level 2 (Team) architecture.
/// Each domain declares which other domains it may depend on — any
/// cross-domain import not listed in `allows` will be flagged by
/// `viontin check --arch`.
///
/// # Examples
///
/// ```ignore
/// use viontin::domain;
///
/// domain!(billing, allows: [order, payment]);
///
/// domain!(order, allows: [payment]);
///
/// domain!(payment, allows: []);
/// ```
#[cfg(feature = "domain")]
#[macro_export]
macro_rules! domain {
    ($name:ident, allows: [$($allow:ident),* $(,)?]) => {{
        static ALLOWS: &[&str] = &[$(stringify!($allow)),*];
        static PROVIDES: &[&str] = &[];
        let d = $crate::Domain {
            name: stringify!($name),
            allows: ALLOWS,
            provides: PROVIDES,
        };
        $crate::register_domain(d);
        d
    }};
    ($name:ident, allows: [$($allow:ident),* $(,)?], provides: [$($prov:ident),* $(,)?]) => {{
        static ALLOWS: &[&str] = &[$(stringify!($allow)),*];
        static PROVIDES: &[&str] = &[$(stringify!($prov)),*];
        let d = $crate::Domain {
            name: stringify!($name),
            allows: ALLOWS,
            provides: PROVIDES,
        };
        $crate::register_domain(d);
        d
    }};
}


