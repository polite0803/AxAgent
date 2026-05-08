failures:

---- content_synthesizer::tests::test_synthesize_section_without_description stdout ----

thread 'content_synthesizer::tests::test_synthesize_section_without_description' (19355) panicked at crates/agent/src/content_synthesizer.rs:414:9:
assertion failed: !content.contains('*')
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::content_synthesizer::tests::test_synthesize_section_without_description::{{closure}}
             at ./src/content_synthesizer.rs:414:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::content_synthesizer::tests::test_synthesize_section_without_description
             at ./src/content_synthesizer.rs:414:40
  26: axagent_agent::content_synthesizer::tests::test_synthesize_section_without_description::{{closure}}
             at ./src/content_synthesizer.rs:406:59
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- credibility_evaluator::tests::test_evaluate_with_recent_date stdout ----

thread 'credibility_evaluator::tests::test_evaluate_with_recent_date' (19412) panicked at crates/agent/src/credibility_evaluator.rs:665:9:
assertion failed: assessment.credibility.recency >= 0.9
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::credibility_evaluator::tests::test_evaluate_with_recent_date::{{closure}}
             at ./src/credibility_evaluator.rs:665:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::credibility_evaluator::tests::test_evaluate_with_recent_date
             at ./src/credibility_evaluator.rs:665:55
  26: axagent_agent::credibility_evaluator::tests::test_evaluate_with_recent_date::{{closure}}
             at ./src/credibility_evaluator.rs:655:46
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- lint_checker::tests::test_check_structure_has_sections_ok stdout ----

thread 'lint_checker::tests::test_check_structure_has_sections_ok' (19928) panicked at crates/agent/src/lint_checker.rs:813:9:
assertion failed: !issues.iter().any(|i| i.code == "content-too-short")
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::lint_checker::tests::test_check_structure_has_sections_ok::{{closure}}
             at ./src/lint_checker.rs:813:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::lint_checker::tests::test_check_structure_has_sections_ok
             at ./src/lint_checker.rs:813:71
  26: axagent_agent::lint_checker::tests::test_check_structure_has_sections_ok::{{closure}}
             at ./src/lint_checker.rs:806:52
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- metrics::tests::test_timed_guard_manual_finish_prevents_double stdout ----

thread 'metrics::tests::test_timed_guard_manual_finish_prevents_double' (19990) panicked at crates/agent/src/metrics.rs:763:9:
assertion `left == right` failed
  left: 0.0
 right: 10.0
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::assert_failed_inner
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:439:17
   3: core::panicking::assert_failed
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:394:5
   4: axagent_agent::metrics::tests::test_timed_guard_manual_finish_prevents_double
             at ./src/metrics.rs:763:9
   5: axagent_agent::metrics::tests::test_timed_guard_manual_finish_prevents_double::{{closure}}
             at ./src/metrics.rs:756:56
   6: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
   7: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- research_agent::tests::test_research_agent_format_sources_for_llm stdout ----

thread 'research_agent::tests::test_research_agent_format_sources_for_llm' (20241) panicked at crates/agent/src/research_agent.rs:944:9:
assertion failed: formatted.contains("web")
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::research_agent::tests::test_research_agent_format_sources_for_llm::{{closure}}
             at ./src/research_agent.rs:944:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::research_agent::tests::test_research_agent_format_sources_for_llm
             at ./src/research_agent.rs:944:43
  26: axagent_agent::research_agent::tests::test_research_agent_format_sources_for_llm::{{closure}}
             at ./src/research_agent.rs:932:58
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- session_manager::tests::test_session_manager_session_last_access_updated stdout ----

thread 'session_manager::tests::test_session_manager_session_last_access_updated' (20638) panicked at crates/agent/src/session_manager.rs:1508:9:
assertion failed: last_access.contains_key(&session_id)
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::session_manager::tests::test_session_manager_session_last_access_updated::{{closure}}
             at ./src/session_manager.rs:1508:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::session_manager::tests::test_session_manager_session_last_access_updated
             at ./src/session_manager.rs:1508:55
  26: axagent_agent::session_manager::tests::test_session_manager_session_last_access_updated::{{closure}}
             at ./src/session_manager.rs:1497:64
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- source_validator::tests::test_get_source_type_from_domain_known stdout ----

thread 'source_validator::tests::test_get_source_type_from_domain_known' (20665) panicked at crates/agent/src/source_validator.rs:739:9:
assertion `left == right` failed
  left: None
 right: Some(Wikipedia)
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::assert_failed_inner
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:439:17
   3: core::panicking::assert_failed
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:394:5
   4: axagent_agent::source_validator::tests::test_get_source_type_from_domain_known
             at ./src/source_validator.rs:739:9
   5: axagent_agent::source_validator::tests::test_get_source_type_from_domain_known::{{closure}}
             at ./src/source_validator.rs:729:48
   6: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
   7: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- source_validator::tests::test_validate_batch stdout ----

thread 'source_validator::tests::test_validate_batch' (20689) panicked at crates/agent/src/source_validator.rs:277:58:
Cannot start a runtime from within a runtime. This happens because a function (like `block_on`) attempted to block the current thread while the thread is being used to drive asynchronous tasks.
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:68:5
   3: tokio::runtime::handle::Handle::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/handle.rs:368:9
   4: tokio::runtime::handle::Handle::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/handle.rs:347:18
   5: axagent_agent::source_validator::SourceValidator::validate_batch::{{closure}}
             at ./src/source_validator.rs:277:58
   6: core::iter::adapters::map::map_fold::{{closure}}
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/iter/adapters/map.rs:88:28
   7: <core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::fold
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/slice/iter/macros.rs:279:27
   8: <core::iter::adapters::map::Map<I,F> as core::iter::traits::iterator::Iterator>::fold
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/iter/adapters/map.rs:128:19
   9: core::iter::traits::iterator::Iterator::for_each
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/iter/traits/iterator.rs:845:14
  10: alloc::vec::Vec<T,A>::extend_trusted
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/alloc/src/vec/mod.rs:4001:26
  11: <alloc::vec::Vec<T,A> as alloc::vec::spec_extend::SpecExtend<T,I>>::spec_extend
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/alloc/src/vec/spec_extend.rs:27:14
  12: <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/alloc/src/vec/spec_from_iter_nested.rs:60:16
  13: <alloc::vec::Vec<T> as alloc::vec::spec_from_iter::SpecFromIter<T,I>>::from_iter
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/alloc/src/vec/spec_from_iter.rs:33:9
  14: <alloc::vec::Vec<T> as core::iter::traits::collect::FromIterator<T>>::from_iter
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/alloc/src/vec/mod.rs:3865:9
  15: core::iter::traits::iterator::Iterator::collect
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/iter/traits/iterator.rs:2064:9
  16: axagent_agent::source_validator::SourceValidator::validate_batch
             at ./src/source_validator.rs:278:14
  17: axagent_agent::source_validator::tests::test_validate_batch::{{closure}}
             at ./src/source_validator.rs:763:33
  18: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
  19: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
  20: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
  21: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
  22: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
  23: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  24: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  25: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  26: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  27: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  28: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  29: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  30: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  31: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  32: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  33: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  34: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  35: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  36: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  37: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  38: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  39: axagent_agent::source_validator::tests::test_validate_batch
             at ./src/source_validator.rs:767:37
  40: axagent_agent::source_validator::tests::test_validate_batch::{{closure}}
             at ./src/source_validator.rs:756:35
  41: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  42: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- trajectory_recorder::tests::test_trajectory_recorder_compute_quality_failure stdout ----

thread 'trajectory_recorder::tests::test_trajectory_recorder_compute_quality_failure' (20968) panicked at crates/agent/src/trajectory_recorder.rs:1155:9:
assertion `left == right` failed
  left: 1.0
 right: 0.0
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::assert_failed_inner
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:439:17
   3: core::panicking::assert_failed
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:394:5
   4: axagent_agent::trajectory_recorder::tests::test_trajectory_recorder_compute_quality_failure::{{closure}}
             at ./src/trajectory_recorder.rs:1155:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   7: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   8: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   9: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
  10: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  11: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  12: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  13: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  14: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  15: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  16: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  17: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  18: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  19: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  20: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  21: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  22: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  23: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  24: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  25: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  26: axagent_agent::trajectory_recorder::tests::test_trajectory_recorder_compute_quality_failure
             at ./src/trajectory_recorder.rs:1156:56
  27: axagent_agent::trajectory_recorder::tests::test_trajectory_recorder_compute_quality_failure::{{closure}}
             at ./src/trajectory_recorder.rs:1146:64
  28: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  29: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- trajectory_recorder::tests::test_trajectory_recorder_determine_outcome_failure_on_error stdout ----

thread 'trajectory_recorder::tests::test_trajectory_recorder_determine_outcome_failure_on_error' (20972) panicked at crates/agent/src/trajectory_recorder.rs:1115:9:
assertion `left == right` failed
  left: Success
 right: Failure
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::assert_failed_inner
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:439:17
   3: core::panicking::assert_failed
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:394:5
   4: axagent_agent::trajectory_recorder::tests::test_trajectory_recorder_determine_outcome_failure_on_error::{{closure}}
             at ./src/trajectory_recorder.rs:1115:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   7: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   8: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   9: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
  10: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  11: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  12: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  13: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  14: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  15: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  16: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  17: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  18: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  19: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  20: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  21: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  22: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  23: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  24: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  25: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  26: axagent_agent::trajectory_recorder::tests::test_trajectory_recorder_determine_outcome_failure_on_error
             at ./src/trajectory_recorder.rs:1115:61
  27: axagent_agent::trajectory_recorder::tests::test_trajectory_recorder_determine_outcome_failure_on_error::{{closure}}
             at ./src/trajectory_recorder.rs:1106:75
  28: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  29: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- wiki_compiler::tests::test_calculate_quality_score_combined_deductions stdout ----

thread 'wiki_compiler::tests::test_calculate_quality_score_combined_deductions' (21058) panicked at crates/agent/src/wiki_compiler.rs:1319:9:
assertion `left == right` failed
  left: 0.30000000000000004
 right: 0.0
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::assert_failed_inner
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:439:17
   3: core::panicking::assert_failed
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:394:5
   4: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_combined_deductions::{{closure}}
             at ./src/wiki_compiler.rs:1319:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   7: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   8: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   9: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
  10: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  11: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  12: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  13: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  14: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  15: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  16: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  17: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  18: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  19: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  20: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  21: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  22: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  23: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  24: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  25: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  26: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_combined_deductions
             at ./src/wiki_compiler.rs:1319:31
  27: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_combined_deductions::{{closure}}
             at ./src/wiki_compiler.rs:1310:64
  28: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  29: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- wiki_compiler::tests::test_calculate_quality_score_no_wikilinks stdout ----

thread 'wiki_compiler::tests::test_calculate_quality_score_no_wikilinks' (21062) panicked at crates/agent/src/wiki_compiler.rs:1254:9:
assertion failed: (score - 0.9).abs() < f64::EPSILON
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_no_wikilinks::{{closure}}
             at ./src/wiki_compiler.rs:1254:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_no_wikilinks
             at ./src/wiki_compiler.rs:1254:52
  26: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_no_wikilinks::{{closure}}
             at ./src/wiki_compiler.rs:1244:57
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- wiki_compiler::tests::test_calculate_quality_score_short_content stdout ----

thread 'wiki_compiler::tests::test_calculate_quality_score_short_content' (21066) panicked at crates/agent/src/wiki_compiler.rs:1240:9:
assertion failed: score <= 0.6
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_short_content::{{closure}}
             at ./src/wiki_compiler.rs:1240:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_short_content
             at ./src/wiki_compiler.rs:1240:30
  26: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_short_content::{{closure}}
             at ./src/wiki_compiler.rs:1230:58
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- wiki_compiler::tests::test_calculate_quality_score_uncertain_language_english stdout ----

thread 'wiki_compiler::tests::test_calculate_quality_score_uncertain_language_english' (21070) panicked at crates/agent/src/wiki_compiler.rs:1267:9:
assertion failed: score < 0.5
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:80:14
   2: core::panicking::panic
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/panicking.rs:150:5
   3: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_uncertain_language_english::{{closure}}
             at ./src/wiki_compiler.rs:1267:9
   4: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:70
   7: tokio::task::coop::with_budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:778:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:451:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:777:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context.rs:181:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:865:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:765:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:205:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/scheduler/current_thread/mod.rs:193:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.1/src/runtime/runtime.rs:345:18
  25: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_uncertain_language_english
             at ./src/wiki_compiler.rs:1267:29
  26: axagent_agent::wiki_compiler::tests::test_calculate_quality_score_uncertain_language_english::{{closure}}
             at ./src/wiki_compiler.rs:1258:71
  27: core::ops::function::FnOnce::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
  28: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    content_synthesizer::tests::test_synthesize_section_without_description
    credibility_evaluator::tests::test_evaluate_with_recent_date
    lint_checker::tests::test_check_structure_has_sections_ok
    metrics::tests::test_timed_guard_manual_finish_prevents_double
    research_agent::tests::test_research_agent_format_sources_for_llm
    session_manager::tests::test_session_manager_session_last_access_updated
    source_validator::tests::test_get_source_type_from_domain_known
    source_validator::tests::test_validate_batch
    trajectory_recorder::tests::test_trajectory_recorder_compute_quality_failure
    trajectory_recorder::tests::test_trajectory_recorder_determine_outcome_failure_on_error
    wiki_compiler::tests::test_calculate_quality_score_combined_deductions
    wiki_compiler::tests::test_calculate_quality_score_no_wikilinks
    wiki_compiler::tests::test_calculate_quality_score_short_content
    wiki_compiler::tests::test_calculate_quality_score_uncertain_language_english

test result: FAILED. 1795 passed; 14 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.14s

error: test failed, to rerun pass `-p axagent-agent --lib`
Error: Process completed with exit code 101.