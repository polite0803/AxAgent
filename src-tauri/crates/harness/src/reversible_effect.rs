// SPDX-License-Identifier: AGPL-3.0-only

//! 可逆效果原语 — 通用「注册即记录、卸载即回滚」的事务式副作用管理。
//!
//! 这是「一切皆插件」重构中动态增删能力（内置实现与外部插件平权）的前提：
//! 任何注册到 [`EffectScope`] 的副作用都携带一个撤销闭包，支持单条撤销与整组逆序回滚。
//!
//! 与 `tool.rs` 中工具级回滚（`Tool::create_rollback_before` / `execute_rollback`）不同，
//! 本模块提供**通用**、与具体工具无关的可逆抽象，可覆盖注册表条目、事件订阅、
//! 服务挂载等任意副作用。

use std::sync::{Arc, Mutex};

/// 可撤销的副作用。
///
/// 实现方提供 [`ReversibleEffect::undo`]，在需要回滚时执行。
/// 所有普通闭包（`Fn() + Send + Sync`）自动实现本 trait，便于用闭包注册撤销逻辑。
pub trait ReversibleEffect: Send + Sync {
    /// 副作用的人类可读名称（用于日志与调试）。
    fn name(&self) -> &str;

    /// 逆序回放该副作用。
    fn undo(&self);
}

/// 任意 `Fn() + Send + Sync` 闭包都天然是可逆效果（撤销即执行闭包）。
impl<T> ReversibleEffect for T
where
    T: Fn() + Send + Sync,
{
    fn name(&self) -> &str {
        "<closure>"
    }

    fn undo(&self) {
        (self)()
    }
}

/// 带命名的可逆效果 — 在闭包基础上附加一个名称，便于日志与去重。
pub struct NamedEffect<F> {
    name: String,
    f: F,
}

impl<F> NamedEffect<F> {
    /// 用名称 + 撤销闭包构造一个可逆效果。
    pub fn new(name: impl Into<String>, f: F) -> Self {
        Self { name: name.into(), f }
    }
}

impl<F> ReversibleEffect for NamedEffect<F>
where
    F: Fn() + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn undo(&self) {
        (self.f)()
    }
}

/// 效果作用域 — 收集一组可逆效果，可逆序整体回滚，也可撤销单条。
///
/// 克隆是廉价的（共享底层存储），因此可方便地分发给多个注册方。
#[derive(Clone)]
pub struct EffectScope {
    inner: Arc<Mutex<Vec<Arc<dyn ReversibleEffect>>>>,
}

/// 一条已注册效果的回滚句柄 — 通过 [`EffectHandle::undo`] 单独撤销。
#[derive(Clone)]
pub struct EffectHandle {
    scope: EffectScope,
    index: usize,
    name: String,
}

impl EffectScope {
    /// 创建一个空作用域。
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(Vec::new())) }
    }

    /// 注册一个可逆效果，返回可单独撤销的句柄。
    pub fn register_effect(&self, effect: Arc<dyn ReversibleEffect>) -> EffectHandle {
        let mut slot = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        slot.push(effect);
        EffectHandle { scope: self.clone(), index: slot.len() - 1, name: "effect".to_string() }
    }

    /// 用名称 + 撤销闭包注册可逆效果（推荐入口）。
    pub fn register(
        &self,
        name: impl Into<String>,
        undo: impl Fn() + Send + Sync + 'static,
    ) -> EffectHandle {
        let handling = NamedEffect::new(name, undo);
        let name = handling.name.to_string();
        let mut slot = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        slot.push(Arc::new(handling));
        EffectHandle { scope: self.clone(), index: slot.len() - 1, name }
    }

    /// 逆序回滚全部已注册效果，并清空作用域。
    pub fn rollback_all(&self) {
        let mut slot = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        for effect in slot.iter().rev() {
            effect.undo();
        }
        slot.clear();
    }

    /// 已注册效果数量。
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for EffectScope {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectHandle {
    /// 撤销该条效果（从作用域移除并回放 `undo`）。
    ///
    /// 注意：若作用域已被 [`EffectScope::rollback_all`] 清空，本操作将变为空操作。
    pub fn undo(&self) {
        let mut slot = self.scope.inner.lock().unwrap_or_else(|e| e.into_inner());
        if self.index < slot.len() {
            let effect = slot.remove(self.index);
            effect.undo();
        }
    }

    /// 效果名称。
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Debug for EffectHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectHandle")
            .field("index", &self.index)
            .field("name", &self.name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn rollback_all_replays_in_reverse_order() {
        let scope = EffectScope::new();
        let order = Arc::new(Mutex::new(Vec::new()));
        let order2 = order.clone();
        scope.register("a", move || order2.lock().unwrap().push("a_undo"));
        let order3 = order.clone();
        scope.register("b", move || order3.lock().unwrap().push("b_undo"));

        scope.rollback_all();

        let seq = order.lock().unwrap().clone();
        assert_eq!(seq, vec!["b_undo", "a_undo"], "应逆序回放");
        assert!(scope.is_empty());
    }

    #[test]
    fn single_handle_undo_removes_only_that_effect() {
        let scope = EffectScope::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let h1 = scope.register("one", move || {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        h1.undo();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(scope.len(), 0);
    }

    #[test]
    fn closure_blanket_impl_works() {
        let scope = EffectScope::new();
        let flag = Arc::new(AtomicUsize::new(0));
        let f = flag.clone();
        let effect: Arc<dyn ReversibleEffect> = Arc::new(move || {
            f.fetch_add(10, Ordering::SeqCst);
        });
        scope.register_effect(effect);
        scope.rollback_all();
        assert_eq!(flag.load(Ordering::SeqCst), 10);
    }
}
