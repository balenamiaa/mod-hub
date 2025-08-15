//! Trait-based hook system with a modular manager and typed configuration.

use core::any::Any;
use core::fmt;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::errors::{Error, Result};
use ilhook::x64::{CallbackOption, HookFlags, HookType, Hooker, Registers};

/// Context passed to modules during initialization.
pub struct HookContext<C: Send + Sync + 'static> {
    config: Arc<RwLock<C>>,
}

impl<C: Send + Sync + 'static> HookContext<C> {
    pub fn config(&self) -> std::sync::RwLockReadGuard<'_, C> {
        self.config.read().unwrap()
    }
    pub fn config_mut(&self) -> std::sync::RwLockWriteGuard<'_, C> {
        self.config.write().unwrap()
    }

    pub unsafe fn install_jmp_back(
        &self,
        target_address: usize,
        callback: unsafe extern "win64" fn(*mut Registers, usize),
        user_data: usize,
    ) -> Result<HookGuard> {
        let hook = unsafe {
            Hooker::new(
                target_address,
                HookType::JmpBack(callback),
                CallbackOption::None,
                user_data,
                HookFlags::empty(),
            )
            .hook()
        }
        .map_err(|e| Error::HookInstall(e))?;
        Ok(HookGuard::own(hook))
    }

    pub unsafe fn install_retn(
        &self,
        target_address: usize,
        callback: unsafe extern "win64" fn(*mut Registers, usize, usize) -> usize,
        user_data: usize,
    ) -> Result<HookGuard> {
        let hook = unsafe {
            Hooker::new(
                target_address,
                HookType::Retn(callback),
                CallbackOption::None,
                user_data,
                HookFlags::empty(),
            )
            .hook()
        }
        .map_err(|e| Error::HookInstall(e))?;
        Ok(HookGuard::own(hook))
    }
}

/// RAII token representing an installed hook.
/// Dropping this value should unhook.
pub struct HookGuard {
    inner: Option<Box<dyn Any>>,
}

impl HookGuard {
    pub fn own<T: 'static>(value: T) -> Self {
        Self {
            inner: Some(Box::new(value)),
        }
    }

    pub fn empty() -> Self {
        Self { inner: None }
    }
}

impl fmt::Debug for HookGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HookGuard").finish_non_exhaustive()
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        let _ = self.inner.take();
    }
}

unsafe impl Send for HookGuard {}

/// A module that can install one or more hooks.
pub trait HookModule<C>: Send + 'static
where
    C: Send + Sync + 'static,
{
    fn name(&self) -> &'static str;

    fn init(&mut self, ctx: &HookContext<C>) -> Result<Vec<HookGuard>>;

    fn shutdown(&mut self) {}
}

/// Manages module registration and hook lifetimes.
pub struct HookManager<C>
where
    C: Send + Sync + 'static,
{
    config: Arc<RwLock<C>>,
    modules: Vec<Box<dyn HookModule<C>>>,
    guards: Vec<HookGuard>,
    started: bool,
}

impl<C> HookManager<C>
where
    C: Send + Sync + 'static,
{
    pub fn new(config: C) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            modules: Vec::new(),
            guards: Vec::new(),
            started: false,
        }
    }

    pub fn config(&self) -> std::sync::RwLockReadGuard<'_, C> {
        self.config.read().unwrap()
    }
    pub fn config_mut(&self) -> std::sync::RwLockWriteGuard<'_, C> {
        self.config.write().unwrap()
    }

    pub fn set_config(&mut self, config: C) {
        *self.config.write().unwrap() = config;
    }

    pub fn register<M>(&mut self, module: M) -> &mut Self
    where
        M: HookModule<C>,
    {
        self.modules.push(Box::new(module));
        self
    }

    pub fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }
        let ctx = HookContext {
            config: self.config.clone(),
        };
        for module in &mut self.modules {
            let mut installed = module.init(&ctx)?;
            self.guards.append(&mut installed);
        }
        self.started = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        if !self.started {
            return;
        }
        for module in &mut self.modules {
            module.shutdown();
        }
        self.guards.clear();
        self.started = false;
    }
}

/// A type-erased global manager so you can plug in your cheat-wide config type.
/// Initialize it once via `init_global_manager(config)`.
static GLOBAL_ANY: OnceLock<Mutex<Box<dyn Any + Send>>> = OnceLock::new();

struct MutexGuarded<C: Send + Sync + 'static> {
    inner: std::sync::MutexGuard<'static, Box<dyn Any + Send>>,
    marker: core::marker::PhantomData<C>,
}

impl<C: Send + Sync + 'static> MutexGuarded<C> {
    fn as_manager(&mut self) -> Option<&mut HookManager<C>> {
        self.inner.downcast_mut::<HookManager<C>>()
    }
}

fn get_manager<C>() -> Option<MutexGuarded<C>>
where
    C: Send + Sync + 'static,
{
    GLOBAL_ANY.get().map(|m| MutexGuarded {
        inner: m.lock().unwrap(),
        marker: core::marker::PhantomData,
    })
}

pub fn init_global_manager<C>(config: C)
where
    C: Send + Sync + 'static,
{
    let _ = GLOBAL_ANY.set(Mutex::new(Box::new(HookManager::<C>::new(config))));
}

pub fn with_manager<C, R>(f: impl FnOnce(&mut HookManager<C>) -> R) -> Option<R>
where
    C: Send + Sync + 'static,
{
    let mut g = get_manager::<C>()?;
    let mgr = g.as_manager()?;
    Some(f(mgr))
}

pub fn register<C, M>(module: M)
where
    C: Send + Sync + 'static,
    M: HookModule<C> + 'static,
{
    let _ = with_manager::<C, _>(|mgr| {
        mgr.register(module);
    });
}

pub fn set_config<C>(config: C)
where
    C: Send + Sync + 'static,
{
    let _ = with_manager::<C, _>(|mgr| mgr.set_config(config));
}

pub fn start<C>() -> Result<()>
where
    C: Send + Sync + 'static,
{
    match with_manager::<C, _>(|mgr| mgr.start()) {
        Some(res) => res,
        None => Ok(()),
    }
}

pub fn stop<C>()
where
    C: Send + Sync + 'static,
{
    let _ = with_manager::<C, _>(|mgr| mgr.stop());
}
