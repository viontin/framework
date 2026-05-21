#[macro_use]
mod inline_macros;
pub mod boot;

pub use boot::{boot, Boot};
pub use viontin_framework as fw;
pub use viontin_macros as macros;
pub use viontin_tui as tui;
pub use viontin_gems as gem;

pub mod app {
    pub use viontin_framework::app::{Application, Container, ServiceProvider};
    pub use viontin_framework::app::{EnvProvider, ConfigProvider, LogProvider, QueueProvider, EventsProvider};
    pub use viontin_framework::route::provider::RouteProvider;
}

#[cfg(feature = "domain")]
pub use viontin_framework::domain::{
    Domain, DomainViolation, DomainEvent, DomainListener,
    GenericDomainEvent, AggregateRoot, Repository,
    register as register_domain, domains, find as find_domain,
    is_allowed as domain_is_allowed, check_all as check_domains, DomainBoundary,
    DomainServiceProvider, DomainConfig,
};

pub use viontin_framework::{
    cache::{Cache, MemoryCache, FileCache, NullCache, CacheDriver},
    cli::{Command, Input, Output, ExitCode, Kernel},
    collection::Collection,
    config::{ConfigRepository as Config, config, config_set, init as config_init, ConfigValue},
    csrf::{CsrfManager, CsrfConfig},
    db::{Connection, ConnectionPool, Value, Row, DbConfig, QueryBuilder},
    debug::{dump, dd, Profiler, benchmark},
    encryption::SimpleEncrypter,
    env::{Environment, load_env, load_env_auto, env, env_int, env_bool, has_env},
    error::{FrameworkError, Result, SourceLocation},
    events::{EventDispatcher, Event, Listener, Subscriber, Subscribable, GenericEvent},
    fs::{read, write, copy, ensure_dir, find_files, TempDir, FileInfo},
    http::{Request, Response, StatusCode, Method, Headers, Uri, Cookie},
    lang::{JsonTranslator, trans, choice, locale, init as lang_init, Translator},
    log::{Logger, StdoutLog, LogFormat, Level, LogEntry, LogChannel, log_info, log_error, log_warning, log_debug},
    middleware::{Middleware, MiddlewareChain},
    mail::{Mailer, Envelope, Attachment, LogTransport, ArrayTransport, Mail},
    notif::{Notifiable, Notification as NotifTrait, Channel as NotificationChannel, MailChannel, DatabaseChannel, Notif},
    page::{Page, PaginationLinks, paginate, simple_paginate},
    queue::{Job, Driver as QueueDriver, SyncQueue, Queue},
    rate::{RateLimiter, RateLimiterDriver, TokenBucketLimiter,
           attempt as rate_attempt, too_many_attempts, remaining as rate_remaining,
           hits as rate_hits, available_in, clear as rate_clear, hit as rate_hit, init as rate_init},
    route::{self as route_registry, RouteRegistry, RouteMethod, RouteDefinition,
            get as route_get, post as route_post, put as route_put, delete as route_delete,
            remove as route_remove, has as route_has, all as route_all, finalize as route_finalize,
            register_handler as route_register_handler,
            provider::RouteProvider},
    schedule::{ScheduledJob, Scheduler, cron_matches},
    semver::{Version, VersionReq, Meta, Compatibility},
    server::{Router, Server, Handler},
    session::{Session, MemorySession, FileSession, SessionDriver},
    storage::{Storage, LocalStorage, MemoryStorage, Driver},
    support::{Hasher, Encrypter, SimpleHasher, truncate, slug, url_decode, url_encode, random},
    testing::{print_arch_result, arch, ArchRule, ArchResult, ArchSeverity, ArchFinding,
           ArchTarget, expect, Expect, ExpectPool, DescribeBuilder, DescribeContext,
           ConsoleReporter, TestReporter, TestEvent, describe, test, it,
           beforeEach, afterEach, beforeAll, afterAll, covers, run_tests,
           TestRunner, TestRunSummary, TestSuite, TestResult, TestStatus},
    validator::{Validator, Outcome, Finding, Severity, Context, ValidatorGroup},
    ws::{Opcode, Message, WebSocketConfig, WebSocketHandler, ws_router, WsRouter, WsServer},
};

pub use viontin_framework::cli::output::Log;
pub use viontin_tui::validator as tui_validator;
pub use viontin_framework::path::{base_path, base_path_glob, url};

pub mod prelude {
    pub use crate::Result;
    pub use crate::Version;
    pub use crate::fw::auth::Auth;
    pub use crate::fw::http::{Request, Response, StatusCode};
    pub use crate::Logger;
    pub use crate::Cache;
    pub use crate::Config;
    pub use crate::{dump, dd};
    pub use crate::Collection;
    pub use crate::EventDispatcher;
    pub use crate::Queue;
    pub use crate::Mail;
    pub use crate::Notif;
    pub use crate::Scheduler;
    pub use crate::RateLimiter;
    pub use crate::TokenBucketLimiter;
    pub use crate::Page;
    pub use crate::SimpleEncrypter;
    pub use crate::fw::cli::{Command, Input, Output, ExitCode, Kernel};
    pub use crate::gem::{GemRegistry, GemKind, GemFacade, GemMeta, GemBinding};
    #[cfg(feature = "domain")]
    pub use crate::Domain;
    #[cfg(feature = "domain")]
    pub use crate::{AggregateRoot, Repository, DomainEvent};
}

// Viontin macros are accessible via `viontin::macros::*`
// Use: `use viontin::macros::domain;` for the #[domain] attribute proc-macro
