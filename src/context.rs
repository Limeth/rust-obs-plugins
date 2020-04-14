use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::mem::ManuallyDrop;

/// During this type's lifetime, certain operations can be performed, that could
/// not be performed otherwise.
///
/// Implementation note:
/// The context should be enabled upon the call to `Context::enter` and left when
/// the type is `Drop::drop`ped.
pub trait Context: Sized {
    /// Either retrieves the context, or tries to enter it.
    /// This is the function the user would most likely want to use.
    ///
    /// Returns `None` on failure.
    fn enter() -> Option<Self> {
        if let Some(current) = Self::get_current() {
            Some(current)
        } else {
            Self::enter_once()
        }
    }

    /// Enters the context only if we are not in the context.
    /// Returns `None` if we are in the context, or if a failure occurred while trying to enter the
    /// context.
    ///
    /// Implementation note:
    /// The inner context **must** be destroyed after being retrieved this way.
    fn enter_once() -> Option<Self>;

    /// The context may be entered in the OBS source code, before handing the execution to our
    /// code via a callback. This function is used to retrieve the current context, in that case.
    /// Returns `None`, if we are not in the context.
    ///
    /// Implementation note:
    /// The inner context **must not** be destroyed after being retrieved this way.
    fn get_current() -> Option<Self>;
}

/// Types influencing the behaviour of `ContextDependent`.
pub trait ContextDependentState {
    fn is_enabled() -> bool;
}

pub struct Enabled<'a, C: Context> {
    context: &'a C,
}

impl<'a, C: Context> ContextDependentState for Enabled<'a, C> {
    fn is_enabled() -> bool { true }
}

pub struct Disabled;

impl ContextDependentState for Disabled {
    fn is_enabled() -> bool { false }
}

/// A wrapper for context-dependent types. Ensures, that operations on this type are only
/// performed within the required context.
///
/// This type can be in two different states, as indicated by the third generic parameter
/// `S: ContextDependentState` -- either `Enabled` or `Disabled`.
///
/// When a context-dependent type is in the `Enabled<'a, C>` state, the wrapped data is made
/// available via `Deref` and `DerefMut`. It is limited to the lifetime `'a` of the context `C`.
/// This lifetime may be escaped, which is especially useful for storing these context-dependent
/// types, using `ContextDependent::disable`.
///
/// When a context-dependent type is in the `Disabled` state, the wrapped data is not accessible,
/// making it impossible to perform context-dependent operations provided by the wrapped type
/// outside of the required context. A disabled context-dependent type is useful for being stored.
/// A disabled context-dependent type may be re-enabled using `ContextDependent::enable`.
///
/// **Warning**:
/// Care should be taken not to `Drop::drop` the disabled type, as destructors are assumed to
/// be context-dependent. Dropping a disabled context-dependent type may cause a `panic!`,
/// if the required context cannot be re-entered.
/// Users should ensure all context-dependent types are dropped during the enabled state.
///
/// Implementation note:
/// It must be ensured that all context-dependent types `T` can only be constructed in a way, so that
/// they are wrapped in this `ContextDepdendent<T, ...>` wrapper, and no other way.
/// Avoid implementing `Clone` on the unwrapped, context-dependent type `T`.
/// Calls to `Drop::drop(&mut T)` are assumed to happen within the context (see
/// `ContextDepdendent::drop` for how that is ensured).
pub struct ContextDependent<T, C: Context, S: ContextDependentState> {
    /// Set to `None` only during state transitions and `Self::map`
    data: Option<ManuallyDrop<T>>,
    state: S,
    __marker: PhantomData<C>,
}

impl<T, C: Context, S: ContextDependentState> ContextDependent<T, C, S> {
    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<'a, T, C: Context> ContextDependent<T, C, Enabled<'a, C>> {
    pub fn new(data: T, context: &'a C) -> Self {
        Self {
            data: Some(ManuallyDrop::new(data)),
            state: Enabled {
                context,
            },
            __marker: Default::default(),
        }
    }

    #[must_use = "A disabled context-dependent type must not be dropped."]
    pub fn disable(mut self) -> ContextDependent<T, C, Disabled> {
        ContextDependent {
            data: self.data.take(),
            state: Disabled,
            __marker: Default::default(),
        }
    }

    pub fn map<R>(mut self, map: impl FnOnce(T) -> R) -> ContextDependent<R, C, Enabled<'a, C>> {
        let data = ManuallyDrop::into_inner(self.data.take().unwrap());
        let mapped_data = (map)(data);

        ContextDependent {
            data: Some(ManuallyDrop::new(mapped_data)),
            state: Enabled {
                context: self.state.context,
            },
            __marker: Default::default(),
        }
    }

    pub fn context(&self) -> &'a C {
        &self.state.context
    }
}

impl<'a, T, C: Context> Deref for ContextDependent<T, C, Enabled<'a, C>> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data.as_ref().expect("The contents of a context-dependent type have been accessed during a state transition.")
    }
}

impl<'a, T, C: Context> DerefMut for ContextDependent<T, C, Enabled<'a, C>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().expect("The contents of a context-dependent type have been accessed during a state transition.")
    }
}

impl<T, C: Context> ContextDependent<T, C, Disabled> {
    pub fn enable<'a>(mut self, context: &'a C) -> ContextDependent<T, C, Enabled<'a, C>> {
        ContextDependent {
            data: self.data.take(),
            state: Enabled {
                context,
            },
            __marker: Default::default(),
        }
    }

    pub fn as_enabled<'a, 'b>(&'b self, context: &'a C) -> EnableGuard<'a, 'b, T, C> {
        EnableGuard {
            disabled: self,
            context,
        }
    }

    pub fn as_enabled_mut<'a, 'b>(&'b mut self, context: &'a C) -> EnableGuardMut<'a, 'b, T, C> {
        EnableGuardMut {
            disabled: self,
            context,
        }
    }
}

impl<T, C: Context, S: ContextDependentState> Drop for ContextDependent<T, C, S> {
    fn drop(&mut self) {
        if self.data.is_none() {
            return;
        }

        let data = self.data.as_mut().unwrap();

        if S::is_enabled() {
            unsafe {
                ManuallyDrop::drop(data);
            }
        } else {
            eprintln!(
                "A context-dependent disabled value of type `{}` is being dropped outside of the context of type `{}`.",
                std::any::type_name::<T>(),
                std::any::type_name::<C>(),
            );

            if let Some(_context) = C::enter() {
                unsafe {
                    ManuallyDrop::drop(data);
                }
            } else {
                eprintln!(
                    "An attempt to drop a context-dependent disabled value of type `{}` outside of the context of type `{}` failed.",
                    std::any::type_name::<T>(),
                    std::any::type_name::<C>(),
                );

                // Attempt to drop anyway.
                unsafe {
                    ManuallyDrop::drop(data);
                }

                panic!();
            }
        }
    }
}

pub struct EnableGuardMut<'a, 'b, T, C: Context> {
    disabled: &'b mut ContextDependent<T, C, Disabled>,
    context: &'a C,
}

impl<'a, 'b, T, C: Context> EnableGuardMut<'a, 'b, T, C> {
    pub fn context(&self) -> &'a C {
        self.context
    }
}

impl<'a, 'b, T, C: Context> Deref for EnableGuardMut<'a, 'b, T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.disabled.data.as_ref().unwrap()
    }
}

impl<'a, 'b, T, C: Context> DerefMut for EnableGuardMut<'a, 'b, T, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.disabled.data.as_mut().unwrap()
    }
}

pub struct EnableGuard<'a, 'b, T, C: Context> {
    disabled: &'b ContextDependent<T, C, Disabled>,
    context: &'a C,
}

impl<'a, 'b, T, C: Context> EnableGuard<'a, 'b, T, C> {
    pub fn context(&self) -> &'a C {
        self.context
    }
}

impl<'a, 'b, T, C: Context> Deref for EnableGuard<'a, 'b, T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.disabled.data.as_ref().unwrap()
    }
}

pub trait DefaultInContext<C: Context>: Sized {
    fn default_in_context<'a>(context: &'a C) -> ContextDependent<Self, C, Enabled<'a, C>>;
}

/// Despite context-dependent types `T` should not implement `Default`, this implementation
/// is still useful for non-context-dependent types.
impl<T: Default, C: Context> DefaultInContext<C> for T {
    fn default_in_context<'a>(context: &'a C) -> ContextDependent<Self, C, Enabled<'a, C>> {
        ContextDependent::new(T::default(), context)
    }
}

pub trait ContextCarrier<C: Context> {
    fn context(&self) -> &C;
}

impl<C: Context> ContextCarrier<C> for C {
    fn context(&self) -> &C {
        self
    }
}
