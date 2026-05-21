use std::path::Path;
use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

fn to_pascal(s: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' { upper = true; }
        else if upper { out.push(c.to_ascii_uppercase()); upper = false; }
        else { out.push(c); }
    }
    out
}

fn to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c == '-' || c == ' ' { out.push('_'); }
        else if c.is_uppercase() && i > 0 { out.push('_'); out.push(c.to_ascii_lowercase()); }
        else { out.push(c.to_ascii_lowercase()); }
    }
    out
}

fn to_kebab(s: &str) -> String { to_snake(s).replace('_', "-") }

// ── Scaffold abstraction ──

pub struct ScaffoldType {
    pub sig: &'static str,
    pub desc: &'static str,
    pub dir: &'static str,
    pub template: fn(&str, &str) -> String,
    pub usage: fn(&str, &str) -> String,
    pub is_domain: bool,
}

pub struct MakeScaffoldCommand {
    pub scaffold: &'static ScaffoldType,
}

impl Command for MakeScaffoldCommand {
    fn signature(&self) -> &str { self.scaffold.sig }
    fn description(&self) -> &str { self.scaffold.desc }
    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let name = match input.argument::<String>("name") {
            Ok(n) => n, Err(e) => { output.error(&e); return ExitCode::InvalidArgs; }
        };
        let force = input.flag("force");
        let current_dir = match std::env::current_dir() {
            Ok(d) => d, Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found");
            return ExitCode::Failure;
        }

        let pascal = to_pascal(&name);
        let snake = to_snake(&name);

        if self.scaffold.is_domain {
            return scaffold_domain(&current_dir, &pascal, &snake, force, output);
        }

        let is_module = self.scaffold.dir.is_empty();
        let target_dir = if is_module {
            current_dir.join("src").join(&snake)
        } else {
            current_dir.join("src").join(self.scaffold.dir)
        };
        let file_name = if is_module { "mod.rs".to_string() } else { format!("{}.rs", snake) };
        let content = (self.scaffold.template)(&pascal, &snake);
        let file_path = target_dir.join(&file_name);

        let cmd_name = self.scaffold.sig.split_whitespace().next().unwrap_or(self.scaffold.sig);
        output.title(cmd_name);
        output.info(&format!("{} ({})", pascal, snake));
        output.line("");

        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            output.error(&format!("Failed to create directory: {}", e));
            return ExitCode::Failure;
        }

        if file_path.exists() && !force {
            output.error(&format!("Already exists: {}", file_path.display()));
            output.line("  Use --force to overwrite");
            return ExitCode::InvalidArgs;
        }

        if let Err(e) = std::fs::write(&file_path, &content) {
            output.error(&format!("Failed to write: {}", e));
            return ExitCode::Failure;
        }
        output.success(&format!("Created: {}", file_path.display()));

        if !is_module {
            let mf = target_dir.join("mod.rs");
            if !mf.exists() {
                std::fs::write(&mf, format!("pub mod {};\n", snake)).ok();
                output.success(&format!("Created: {}", mf.display()));
            } else if !mod_has_entry(&mf, &snake) {
                if let Ok(c) = std::fs::read_to_string(&mf) {
                    std::fs::write(&mf, format!("{}\npub mod {};\n", c.trim_end(), snake)).ok();
                    output.success(&format!("Updated: {}", mf.display()));
                }
            }
        }

        output.line("");
        let hint = (self.scaffold.usage)(&pascal, &snake);
        output.info(&hint);

        ExitCode::Success
    }
}

fn scaffold_domain(current_dir: &std::path::Path, pascal: &str, snake: &str, force: bool, output: &Output) -> ExitCode {
    let domain_dir = current_dir.join("src").join("domain").join(snake);

    output.title("make:domain");
    output.info(&format!("{} ({})", pascal, snake));
    output.line("");

    if domain_dir.exists() && !force {
        output.error(&format!("Domain already exists: {}", domain_dir.display()));
        output.line("  Use --force to overwrite");
        return ExitCode::InvalidArgs;
    }

    if let Err(e) = std::fs::create_dir_all(&domain_dir) {
        output.error(&format!("Failed to create directory: {}", e));
        return ExitCode::Failure;
    }

    let domain_rs = domain_dir.join("domain.rs");
    if !domain_rs.exists() || force {
        let content = tpl_domain(pascal, snake);
        std::fs::write(&domain_rs, content).map_err(|e| {
            output.error(&format!("Failed to write domain.rs: {}", e));
        }).ok();
        output.success(&format!("Created: {}", domain_rs.display()));
    }

    let port_rs = domain_dir.join("port.rs");
    if !port_rs.exists() || force {
        let content = format!("// {pascal} — public API (ports)\n//\n// Expose only what other domains need.\n// Keep internal details private to maintain the boundary.\n");
        std::fs::write(&port_rs, content).ok();
        output.success(&format!("Created: {}", port_rs.display()));
    }

    let mod_rs = domain_dir.join("mod.rs");
    if !mod_rs.exists() || force {
        let content = format!("pub mod domain;\npub mod port;\n");
        std::fs::write(&mod_rs, content).ok();
        output.success(&format!("Created: {}", mod_rs.display()));
    }

    let domain_parent_mod = current_dir.join("src").join("domain").join("mod.rs");
    if !domain_parent_mod.exists() {
        std::fs::write(&domain_parent_mod, format!("pub mod {snake};\n")).ok();
        output.success(&format!("Created: {}", domain_parent_mod.display()));
    } else if !mod_has_entry(&domain_parent_mod, snake) {
        if let Ok(c) = std::fs::read_to_string(&domain_parent_mod) {
            std::fs::write(&domain_parent_mod, format!("{}\npub mod {};\n", c.trim_end(), snake)).ok();
            output.success(&format!("Updated: {}", domain_parent_mod.display()));
        }
    }

    output.line("");
    output.info(&hint_domain(pascal, snake));

    ExitCode::Success
}

fn mod_has_entry(mod_file: &Path, name: &str) -> bool {
    if let Ok(c) = std::fs::read_to_string(mod_file) {
        c.lines().any(|l| l.trim() == &format!("pub mod {};", name))
    } else { false }
}

// ── Templates ──

fn tpl_model(pascal: &str, _snake: &str) -> String {
    format!(r##"use serde::{{Serialize, Deserialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {pascal} {{
    pub id: i64,
}}
"##)
}

fn tpl_route(pascal: &str, _snake: &str) -> String {
    format!(r##"use viontin::{{Request, Response}};

pub fn index(_req: Request) -> Response {{
    Response::html("<h1>{pascal}</h1>")
}}
"##)
}

fn tpl_command(pascal: &str, snake: &str) -> String {
    let kebab = to_kebab(snake);
    format!(r##"use viontin_tui::{{Command, Input, Output, ExitCode}};

pub struct {pascal};

impl Command for {pascal} {{
    fn signature(&self) -> &str {{ "{kebab}" }}
    fn description(&self) -> &str {{ "Describe what this command does" }}

    fn handle(&self, _input: &Input, output: &Output) -> ExitCode {{
        output.success("Done");
        ExitCode::Success
    }}
}}
"##)
}

fn tpl_event(pascal: &str, _snake: &str) -> String {
    format!(r##"use serde::{{Serialize, Deserialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {pascal} {{
    // event payload
}}
"##)
}

fn tpl_job(pascal: &str, snake: &str) -> String {
    let kebab = to_kebab(snake);
    format!(r##"#[derive(Debug)]
pub struct {pascal};

impl {pascal} {{
    pub fn handle(self) -> Result<(), String> {{
        Ok(())
    }}

    pub fn name(&self) -> &str {{
        "{kebab}"
    }}
}}
"##)
}

fn tpl_mail(pascal: &str, _snake: &str) -> String {
    format!(r##"use viontin::fw::mail::Envelope;

pub fn build(to: &str) -> Envelope {{
    Envelope {{
        from: "hello@example.com".into(),
        to: vec![to.into()],
        subject: "Subject".into(),
        html_body: "<h1>Hello from {pascal}</h1>".into(),
        text_body: "Hello from {pascal}".into(),
        ..Default::default()
    }}
}}
"##)
}

fn tpl_notification(pascal: &str, _snake: &str) -> String {
    format!(r##"use viontin::fw::notif::{{Notification, Notifiable}};

#[derive(Debug, Clone)]
pub struct {pascal};

impl Notification for {pascal} {{
    fn channels(&self) -> Vec<&'static str> {{ vec!["mail"] }}

    fn to_mail(&self, _notifiable: &dyn Notifiable) -> Option<String> {{
        Some("Notification body".into())
    }}
}}
"##)
}

fn tpl_query(_pascal: &str, _snake: &str) -> String {
    format!(r##"pub fn execute() -> Result<Vec<String>, String> {{
    Ok(vec![])
}}
"##)
}

fn tpl_module(pascal: &str, _snake: &str) -> String {
    format!(r##"// {pascal} module

pub fn init() {{
    // module initialization
}}
"##)
}

fn tpl_domain(pascal: &str, snake: &str) -> String {
    format!(r##"use viontin::Domain;

/// {pascal} domain definition.
///
/// The `allows` list declares which other domains this domain
/// may import from. Any cross-domain import not listed here
/// will be flagged by `viontin check --arch`.
pub const DEFINITION: Domain = Domain::new("{snake}")
    .allows(&[]);

// ── Public API (ports) ──
// Expose only what other domains need to use from here.
// Keep internal details private to maintain the boundary.
"##)
}

// ── Usage hints ──

fn hint_model(_pascal: &str, _snake: &str) -> String {
    "Use it with: models::YourModel".to_string()
}

fn hint_route(pascal: &str, _snake: &str) -> String {
    format!("Register in Router: .get(\"/\", Arc::new({pascal}::index))")
}

fn hint_command(pascal: &str, snake: &str) -> String {
    format!("Register in Kernel: .register(commands::{snake}::{pascal})")
}

fn hint_event(pascal: &str, _snake: &str) -> String {
    format!("Use in EventDispatcher: dispatcher.dispatch(&{pascal} {{...}})")
}

fn hint_job(_pascal: &str, _snake: &str) -> String {
    "Queue it with: queue.push(MyJob)".to_string()
}

fn hint_mail(pascal: &str, _snake: &str) -> String {
    format!("Send with: Mail::new(transport).send({pascal}::build(\"user@example.com\"))")
}

fn hint_notification(pascal: &str, _snake: &str) -> String {
    format!("Notify with: notif.send(&user, &{pascal})")
}

fn hint_query(pascal: &str, _snake: &str) -> String {
    format!("Call with: {pascal}::execute()")
}

fn hint_module(_pascal: &str, snake: &str) -> String {
    format!("Import with: mod {snake};")
}

fn hint_domain(_pascal: &str, snake: &str) -> String {
    format!("Register in main.rs and run \x1b[33mviontin check --arch\x1b[0m to verify boundaries.\n  Edit src/domain/{snake}/domain.rs to declare allowed dependencies.")
}

// ── Scaffold registry ──

pub static CONTROLLER: ScaffoldType = ScaffoldType {
    sig: "make:controller {name} {--force}",
    desc: "Scaffold a new controller",
    dir: "controllers",
    template: |p, _| format!(r##"use viontin::{{Request, Response}};

pub fn index(_req: Request) -> Response {{
    Response::html("<h1>{p}</h1>")
}}
"##),
    usage: |p, _| format!("Register route: .get(\"/\", Arc::new({p}::index))"),
    is_domain: false,
};

pub static MIDDLEWARE: ScaffoldType = ScaffoldType {
    sig: "make:middleware {name} {--force}",
    desc: "Scaffold a new middleware",
    dir: "middleware",
    template: |p, _s| format!(r##"use viontin::{{Request, Response}};

pub struct {p};

impl {p} {{
    pub fn handle(req: Request, next: fn(Request) -> Response) -> Response {{
        next(req)
    }}
}}
"##),
    usage: |_p, _| format!("Use with: Router middleware chain"),
    is_domain: false,
};

pub static MODEL: ScaffoldType = ScaffoldType {
    sig: "make:model {name} {--force}",
    desc: "Scaffold a new model",
    dir: "models",
    template: tpl_model,
    usage: hint_model,
    is_domain: false,
};

pub static ROUTE: ScaffoldType = ScaffoldType {
    sig: "make:route {name} {--force}",
    desc: "Scaffold a new route handler",
    dir: "routes",
    template: tpl_route,
    usage: hint_route,
    is_domain: false,
};

pub static COMMAND: ScaffoldType = ScaffoldType {
    sig: "make:command {name} {--force}",
    desc: "Scaffold a new CLI command",
    dir: "commands",
    template: tpl_command,
    usage: hint_command,
    is_domain: false,
};

pub static EVENT: ScaffoldType = ScaffoldType {
    sig: "make:event {name} {--force}",
    desc: "Scaffold a new event",
    dir: "events",
    template: tpl_event,
    usage: hint_event,
    is_domain: false,
};

pub static JOB: ScaffoldType = ScaffoldType {
    sig: "make:job {name} {--force}",
    desc: "Scaffold a new job",
    dir: "jobs",
    template: tpl_job,
    usage: hint_job,
    is_domain: false,
};

pub static MAIL: ScaffoldType = ScaffoldType {
    sig: "make:mail {name} {--force}",
    desc: "Scaffold a new mail template",
    dir: "mail",
    template: tpl_mail,
    usage: hint_mail,
    is_domain: false,
};

pub static NOTIFICATION: ScaffoldType = ScaffoldType {
    sig: "make:notification {name} {--force}",
    desc: "Scaffold a new notification",
    dir: "notifications",
    template: tpl_notification,
    usage: hint_notification,
    is_domain: false,
};

pub static QUERY: ScaffoldType = ScaffoldType {
    sig: "make:query {name} {--force}",
    desc: "Scaffold a new query",
    dir: "queries",
    template: tpl_query,
    usage: hint_query,
    is_domain: false,
};

pub static MODULE: ScaffoldType = ScaffoldType {
    sig: "make:module {name} {--force}",
    desc: "Scaffold a new module with mod.rs",
    dir: "",
    template: tpl_module,
    usage: hint_module,
    is_domain: false,
};

pub static DOMAIN: ScaffoldType = ScaffoldType {
    sig: "make:domain {name} {--force}",
    desc: "Scaffold a new domain with boundary definition",
    dir: "domain",
    template: tpl_domain,
    usage: hint_domain,
    is_domain: true,
};