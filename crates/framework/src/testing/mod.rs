// Test framework — re-exported from viontest (standalone testing crate).
//
// Viontest provides: expect, describe, arch, test runner.
// Framework wraps it so users can access via viontin::testing::*.

pub use viontest::{
    expect, Expect, ExpectPool,
    describe, test, it, covers,
    beforeEach, afterEach, beforeAll, afterAll,
    DescribeBuilder, DescribeContext,
    ConsoleReporter, TestReporter, TestEvent,
    run_describe_tests as run_tests,
    arch, ArchTarget,
    ArchRule, ArchChecker, ArchResult, ArchSeverity, ArchFinding,
    IsPascalCase, IsCamelCase, IsSnakeCase, IsKebabCase,
    DoesNotDependOn, MustDependOn, EndsWith, StartsWith,
    print_arch_result,
    TestRunner, TestRunSummary, TestSuite, TestResult, TestStatus, ConsoleTestReporter as TestRunnerReporter,
};
