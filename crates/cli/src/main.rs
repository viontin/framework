use viontin_tui::Kernel;

mod commands;
mod project;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kernel = Kernel::new()
        .name("Viontin")
        .version("0.1.0")
        // Level 0 — Core
        .register(commands::new::NewCommand)
        .register(commands::pkg::InitCommand)
        .register(commands::build::BuildCommand)
        .register(commands::dev::DevCommand)
        .register(commands::run::RunCommand)
        .register(commands::check::CheckCommand)
        .register(commands::test::TestCommand)
        .register(commands::add::AddCommand)
        // Cargo Management
        .register(commands::pkg::CleanCommand)
        .register(commands::pkg::DocCommand)
        .register(commands::pkg::FixCommand)
        .register(commands::pkg::BenchCommand)
        .register(commands::pkg::TreeCommand)
        .register(commands::pkg::PackageCommand)
        .register(commands::pkg::MetadataCommand)
        // Publishing & Registry
        .register(commands::pkg::PublishCommand)
        .register(commands::pkg::UpdateCommand)
        .register(commands::pkg::InstallCommand)
        .register(commands::pkg::UninstallCommand)
        .register(commands::pkg::SearchCommand)
        // Code Quality
        .register(commands::pkg::FmtCommand)
        .register(commands::pkg::ClippyCommand)
        // Level 1 — Scaffolding
        .register(commands::inspect::InspectCommand)
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::CONTROLLER })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::MIDDLEWARE })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::MODEL })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::ROUTE })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::COMMAND })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::EVENT })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::JOB })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::MAIL })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::NOTIFICATION })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::QUERY })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::MODULE })
        // Level 2 — Domains & DDD
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::DOMAIN })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::AGGREGATE })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::ENTITY })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::VALUE_OBJECT })
        // Architecture Patterns (RSC / MVC)
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::PROVIDER })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::SERVICE })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::REPOSITORY })
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::VIEW })
        // Microservices
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::SERVICE_CONTRACT })
        // General Contracts
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::CONTRACT })
        // Database
        .register(commands::make::MakeScaffoldCommand { scaffold: &commands::make::MIGRATION });
    let code = kernel.run(&args);
    code.exit();
}
