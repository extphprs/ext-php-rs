//! Thread-safe channel for calling PHP functions from async Rust code.
//!
//! This module implements a CSP (Communicating Sequential Processes) pattern
//! for safely invoking PHP callables from multiple threads. Since PHP's runtime
//! is not thread-safe, we cannot send `ZendCallable` or `Zval` across thread
//! boundaries. Instead, we serialize callable requests and execute them on the
//! PHP thread.
//!
//! # Example: Named Functions
//!
//! ```rust,ignore
//! use ext_php_rs::types::callable_channel::{CallableChannel, CallableRequest};
//! use std::sync::Arc;
//!
//! // Create a channel (typically done once at module init)
//! let channel = Arc::new(CallableChannel::new());
//!
//! // From an async context, queue a callable request
//! let channel_clone = channel.clone();
//! tokio::spawn(async move {
//!     let result = channel_clone.call("strtoupper", vec!["hello".into()]).await;
//!     // result contains the stringified return value
//! });
//!
//! // On the PHP thread, process pending requests
//! channel.process_pending();
//! ```
//!
//! # Example: Closures
//!
//! ```rust,ignore
//! use ext_php_rs::types::callable_channel::{CallableChannel, ClosureRegistry};
//! use std::sync::Arc;
//!
//! // On the PHP thread, register a closure and get its ID
//! let registry = ClosureRegistry::global();
//! let closure_id = registry.register(php_closure)?;
//!
//! // The ID is Send + Sync and can be passed to other threads
//! let channel = CallableChannel::global();
//! tokio::spawn(async move {
//!     let handle = channel.queue_closure_call(closure_id, vec!["arg".into()]);
//!     let result = handle.wait();
//! });
//!
//! // On the PHP thread, process the request
//! channel.process_pending();
//!
//! // When done, unregister the closure
//! registry.unregister(closure_id);
//! ```

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::convert::IntoZval;
use crate::error::Result;
use crate::types::{ZendCallable, Zval};

/// A unique identifier for a registered closure.
///
/// This ID is `Send + Sync` and can be safely passed across thread boundaries.
/// Use it with [`ClosureRegistry`] to register closures and with
/// [`CallableChannel::queue_closure_call`] to invoke them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClosureId(u64);

impl ClosureId {
    /// Create a `ClosureId` from a raw u64 value.
    ///
    /// This is useful when storing closure IDs in PHP as integers.
    #[must_use]
    pub const fn from_u64(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw u64 value of this closure ID.
    ///
    /// This is useful when passing closure IDs to PHP as integers.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Registry for storing PHP closures that can be called from other threads.
///
/// Since `ZendCallable` cannot be sent across threads, this registry stores
/// closures on the PHP thread and provides thread-safe IDs that can be used
/// to invoke them later.
///
/// # Thread Safety
///
/// - `register()` and `unregister()` must be called on the PHP thread
/// - `ClosureId` values can be safely sent to other threads
/// - The actual closure execution happens on the PHP thread via `CallableChannel`
pub struct ClosureRegistry {
    closures: Mutex<HashMap<u64, Zval>>,
    next_id: AtomicU64,
}

impl Default for ClosureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ClosureRegistry {
    /// Create a new closure registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            closures: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Register a callable from a Zval reference and return its ID.
    ///
    /// **Must be called on the PHP thread.**
    ///
    /// The Zval is cloned with proper reference counting, so the original
    /// can be freed. The returned `ClosureId` can be safely sent to other
    /// threads and used with [`CallableChannel::queue_closure_call`].
    ///
    /// # Arguments
    ///
    /// * `callable` - A reference to a callable Zval (closure, function name, etc.)
    ///
    /// # Returns
    ///
    /// A unique ID that can be used to invoke this callable, or `None` if
    /// the Zval is not callable.
    pub fn register(&self, callable: &Zval) -> Option<ClosureId> {
        if !callable.is_callable() {
            return None;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // Clone with proper refcount increment
        let owned = callable.shallow_clone();
        self.closures.lock().insert(id, owned);
        Some(ClosureId(id))
    }

    /// Register a callable that is already owned.
    ///
    /// **Must be called on the PHP thread.**
    ///
    /// Use this when you already have an owned Zval (e.g., from
    /// `ZendCallable::try_from_name`).
    ///
    /// # Arguments
    ///
    /// * `callable` - An owned callable Zval
    ///
    /// # Returns
    ///
    /// A unique ID, or `None` if the Zval is not callable.
    pub fn register_owned(&self, callable: Zval) -> Option<ClosureId> {
        if !callable.is_callable() {
            return None;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.closures.lock().insert(id, callable);
        Some(ClosureId(id))
    }

    /// Unregister a closure by its ID.
    ///
    /// **Must be called on the PHP thread.**
    ///
    /// After unregistering, any pending calls to this closure will fail.
    ///
    /// # Returns
    ///
    /// `true` if the closure was found and removed, `false` otherwise.
    pub fn unregister(&self, id: ClosureId) -> bool {
        self.closures.lock().remove(&id.0).is_some()
    }

    /// Check if a closure is registered.
    #[must_use]
    pub fn contains(&self, id: ClosureId) -> bool {
        self.closures.lock().contains_key(&id.0)
    }

    /// Get the number of registered closures.
    #[must_use]
    pub fn len(&self) -> usize {
        self.closures.lock().len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.closures.lock().is_empty()
    }

    /// Execute a registered closure with the given arguments.
    ///
    /// **Must be called on the PHP thread.**
    ///
    /// This is called internally by `CallableChannel::process_pending()`.
    fn execute(
        &self,
        id: ClosureId,
        args: &[&dyn crate::convert::IntoZvalDyn],
    ) -> std::result::Result<Zval, String> {
        let closures = self.closures.lock();
        let zval = closures
            .get(&id.0)
            .ok_or_else(|| format!("Closure {id:?} not found in registry"))?;
        let callable =
            ZendCallable::new(zval).map_err(|e| format!("Stored value is not callable: {e}"))?;
        callable
            .try_call(args.to_vec())
            .map_err(|e| format!("Closure call failed: {e}"))
    }
}

// Note: ClosureRegistry contains Zval values which are NOT Send/Sync.
// It should only be accessed from the PHP thread. The ClosureId values it produces
// ARE Send/Sync and can be passed to other threads.

/// A serialized value that can be safely sent across threads.
///
/// Since `Zval` cannot implement `Send`, we convert values to this
/// intermediate representation for cross-thread communication.
#[derive(Debug, Clone)]
pub enum SerializedValue {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Long(i64),
    /// Float value
    Double(f64),
    /// String value
    String(String),
    /// Array of key-value pairs (keys are strings or integers)
    Array(Vec<(ArrayKey, SerializedValue)>),
}

/// Array key type for serialized arrays.
#[derive(Debug, Clone)]
pub enum ArrayKey {
    /// Integer key
    Int(i64),
    /// String key
    String(String),
}

impl SerializedValue {
    /// Create a `SerializedValue` from a `Zval`.
    ///
    /// Returns `None` if the value cannot be serialized (e.g., objects, resources).
    #[must_use]
    pub fn from_zval(zval: &Zval) -> Option<Self> {
        if zval.is_null() {
            Some(Self::Null)
        } else if zval.is_bool() {
            Some(Self::Bool(zval.bool().unwrap_or(false)))
        } else if zval.is_long() {
            Some(Self::Long(zval.long()?))
        } else if zval.is_double() {
            Some(Self::Double(zval.double()?))
        } else if zval.is_string() {
            Some(Self::String(zval.string()?.clone()))
        } else if zval.is_array() {
            let arr = zval.array()?;
            let mut items = Vec::with_capacity(arr.len());
            for (key, val) in arr {
                let key = match key {
                    crate::types::ArrayKey::Long(i) => ArrayKey::Int(i),
                    crate::types::ArrayKey::String(s) => ArrayKey::String(s.clone()),
                    crate::types::ArrayKey::Str(s) => ArrayKey::String(s.to_string()),
                };
                let val = Self::from_zval(val)?;
                items.push((key, val));
            }
            Some(Self::Array(items))
        } else {
            // Objects, resources, etc. cannot be safely serialized
            None
        }
    }

    /// Convert this serialized value back to a `Zval`.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion fails.
    pub fn to_zval(&self) -> Result<Zval> {
        match self {
            Self::Null => Ok(Zval::new()),
            Self::Bool(b) => b.into_zval(false),
            Self::Long(i) => i.into_zval(false),
            Self::Double(f) => f.into_zval(false),
            Self::String(s) => s.as_str().into_zval(false),
            Self::Array(items) => {
                let mut arr = crate::types::ZendHashTable::new();
                for (key, val) in items {
                    let zval = val.to_zval()?;
                    match key {
                        ArrayKey::Int(i) => arr.insert_at_index(*i, zval)?,
                        ArrayKey::String(s) => arr.insert(s.as_str(), zval)?,
                    }
                }
                arr.into_zval(false)
            }
        }
    }
}

// Implement common From conversions for SerializedValue
impl From<()> for SerializedValue {
    fn from((): ()) -> Self {
        Self::Null
    }
}

impl From<bool> for SerializedValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for SerializedValue {
    fn from(i: i64) -> Self {
        Self::Long(i)
    }
}

impl From<i32> for SerializedValue {
    fn from(i: i32) -> Self {
        Self::Long(i64::from(i))
    }
}

impl From<f64> for SerializedValue {
    fn from(f: f64) -> Self {
        Self::Double(f)
    }
}

impl From<String> for SerializedValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for SerializedValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

/// The target of a callable request.
#[derive(Debug)]
pub enum CallableTarget {
    /// A named function (e.g., `strtoupper`, `MyClass::method`)
    Function(String),
    /// A registered closure ID
    Closure(ClosureId),
}

/// A request to call a PHP function or closure.
#[derive(Debug)]
pub struct CallableRequest {
    /// Unique identifier for this request
    pub id: u64,
    /// The callable target (function name or closure ID)
    pub target: CallableTarget,
    /// Serialized arguments
    pub arguments: Vec<SerializedValue>,
    /// Channel to send the response
    response_tx: std::sync::mpsc::SyncSender<CallableResponse>,
}

/// Response from a PHP function call.
#[derive(Debug, Clone)]
pub struct CallableResponse {
    /// The request ID this response corresponds to
    pub id: u64,
    /// The result of the call
    pub result: std::result::Result<SerializedValue, String>,
}

/// A handle to await the result of a callable request.
pub struct CallableHandle {
    #[allow(dead_code)]
    id: u64,
    response_rx: std::sync::mpsc::Receiver<CallableResponse>,
}

impl CallableHandle {
    /// Block until the result is ready.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is disconnected or the call failed.
    pub fn wait(self) -> std::result::Result<SerializedValue, String> {
        self.response_rx
            .recv()
            .map_err(|e| format!("Channel disconnected: {e}"))?
            .result
    }

    /// Try to get the result without blocking.
    ///
    /// Returns `None` if the result is not ready yet.
    #[must_use]
    pub fn try_get(&self) -> Option<std::result::Result<SerializedValue, String>> {
        self.response_rx.try_recv().ok().map(|r| r.result)
    }
}

/// A thread-safe channel for queueing PHP callable requests.
///
/// This implements the CSP pattern for safely calling PHP functions
/// from async/threaded Rust code.
pub struct CallableChannel {
    /// Queue of pending requests
    pending: Mutex<VecDeque<CallableRequest>>,
    /// Counter for generating unique request IDs
    next_id: AtomicU64,
    /// Registry for closures (stored here for processing)
    closure_registry: ClosureRegistry,
}

impl Default for CallableChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl CallableChannel {
    /// Create a new callable channel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            next_id: AtomicU64::new(1),
            closure_registry: ClosureRegistry::new(),
        }
    }

    /// Get a reference to the closure registry.
    ///
    /// Use this to register closures that can be called from other threads.
    #[must_use]
    pub fn registry(&self) -> &ClosureRegistry {
        &self.closure_registry
    }

    /// Queue a function call request and return a handle to await the result.
    ///
    /// This method is safe to call from any thread. The actual PHP function
    /// call will be executed when `process_pending` is called on the PHP thread.
    ///
    /// # Arguments
    ///
    /// * `function_name` - Name of the PHP function to call
    /// * `arguments` - Arguments to pass to the function
    ///
    /// # Returns
    ///
    /// A handle that can be used to retrieve the result.
    pub fn queue_call(
        &self,
        function_name: impl Into<String>,
        arguments: Vec<SerializedValue>,
    ) -> CallableHandle {
        self.queue_request(CallableTarget::Function(function_name.into()), arguments)
    }

    /// Queue a closure call request and return a handle to await the result.
    ///
    /// This method is safe to call from any thread. The closure must have been
    /// previously registered using [`ClosureRegistry::register`].
    ///
    /// # Arguments
    ///
    /// * `closure_id` - ID of the registered closure
    /// * `arguments` - Arguments to pass to the closure
    ///
    /// # Returns
    ///
    /// A handle that can be used to retrieve the result.
    pub fn queue_closure_call(
        &self,
        closure_id: ClosureId,
        arguments: Vec<SerializedValue>,
    ) -> CallableHandle {
        self.queue_request(CallableTarget::Closure(closure_id), arguments)
    }

    /// Internal method to queue a request with a target.
    fn queue_request(
        &self,
        target: CallableTarget,
        arguments: Vec<SerializedValue>,
    ) -> CallableHandle {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);

        let request = CallableRequest {
            id,
            target,
            arguments,
            response_tx: tx,
        };

        self.pending.lock().push_back(request);

        CallableHandle {
            id,
            response_rx: rx,
        }
    }

    /// Process all pending callable requests.
    ///
    /// **This must be called on the PHP thread.**
    ///
    /// This method executes all queued PHP function calls and sends
    /// the results back through the response channels.
    ///
    /// # Returns
    ///
    /// The number of requests processed.
    pub fn process_pending(&self) -> usize {
        let mut count = 0;
        loop {
            let request = self.pending.lock().pop_front();
            let Some(request) = request else {
                break;
            };

            let result = self.execute_request(&request);
            let response = CallableResponse {
                id: request.id,
                result,
            };

            // Send response (ignore error if receiver dropped)
            let _ = request.response_tx.send(response);
            count += 1;
        }
        count
    }

    /// Check if there are pending requests.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending.lock().is_empty()
    }

    /// Get the number of pending requests.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }

    /// Execute a single callable request.
    fn execute_request(
        &self,
        request: &CallableRequest,
    ) -> std::result::Result<SerializedValue, String> {
        // Convert serialized arguments to Zvals
        let mut zval_args: Vec<Zval> = Vec::with_capacity(request.arguments.len());
        for (i, arg) in request.arguments.iter().enumerate() {
            let zval = arg
                .to_zval()
                .map_err(|e| format!("Failed to convert argument {i}: {e}"))?;
            zval_args.push(zval);
        }

        // Create references for the call
        let arg_refs: Vec<&dyn crate::convert::IntoZvalDyn> = zval_args
            .iter()
            .map(|z| z as &dyn crate::convert::IntoZvalDyn)
            .collect();

        // Execute based on target type
        let result = match &request.target {
            CallableTarget::Function(name) => {
                let callable = ZendCallable::try_from_name(name)
                    .map_err(|e| format!("Failed to resolve function '{name}': {e}"))?;
                callable
                    .try_call(arg_refs)
                    .map_err(|e| format!("Call failed: {e}"))?
            }
            CallableTarget::Closure(id) => self.closure_registry.execute(*id, &arg_refs)?,
        };

        // Serialize the result
        SerializedValue::from_zval(&result).ok_or_else(|| {
            "Failed to serialize return value (objects/resources not supported)".to_string()
        })
    }
}

// Safety: CallableChannel uses interior mutability with parking_lot::Mutex
// which is Send + Sync. The actual PHP calls only happen on the PHP thread
// via process_pending().
unsafe impl Send for CallableChannel {}
unsafe impl Sync for CallableChannel {}

/// A global callable channel instance.
///
/// This can be used when you only need a single channel for your extension.
static GLOBAL_CHANNEL: std::sync::OnceLock<Arc<CallableChannel>> = std::sync::OnceLock::new();

/// Get or initialize the global callable channel.
#[must_use]
pub fn global_channel() -> Arc<CallableChannel> {
    GLOBAL_CHANNEL
        .get_or_init(|| Arc::new(CallableChannel::new()))
        .clone()
}

/// Queue a call on the global channel.
///
/// This is a convenience function for simple use cases.
///
/// # Arguments
///
/// * `function_name` - Name of the PHP function to call
/// * `arguments` - Arguments to pass to the function
pub fn queue_call(
    function_name: impl Into<String>,
    arguments: Vec<SerializedValue>,
) -> CallableHandle {
    global_channel().queue_call(function_name, arguments)
}

/// Process pending calls on the global channel.
///
/// **Must be called on the PHP thread.**
///
/// # Returns
///
/// The number of requests processed.
#[must_use]
pub fn process_pending() -> usize {
    global_channel().process_pending()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialized_value_conversions() {
        // Test basic conversions
        let null: SerializedValue = ().into();
        assert!(matches!(null, SerializedValue::Null));

        let b: SerializedValue = true.into();
        assert!(matches!(b, SerializedValue::Bool(true)));

        let i: SerializedValue = 42i64.into();
        assert!(matches!(i, SerializedValue::Long(42)));

        let f: SerializedValue = 2.5f64.into();
        assert!(matches!(f, SerializedValue::Double(f) if (f - 2.5).abs() < f64::EPSILON));

        let s: SerializedValue = "hello".into();
        assert!(matches!(s, SerializedValue::String(ref s) if s == "hello"));
    }

    #[test]
    fn test_channel_queue() {
        let channel = CallableChannel::new();

        assert!(!channel.has_pending());
        assert_eq!(channel.pending_count(), 0);

        let _handle = channel.queue_call("test_func", vec!["arg1".into()]);

        assert!(channel.has_pending());
        assert_eq!(channel.pending_count(), 1);
    }

    #[test]
    fn test_closure_id_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ClosureId>();
    }

    #[test]
    fn test_channel_queue_closure() {
        let channel = CallableChannel::new();

        // Queue a closure call (we don't have a real closure, just testing the queue)
        let closure_id = ClosureId(42);
        let _handle = channel.queue_closure_call(closure_id, vec!["arg".into()]);

        assert!(channel.has_pending());
        assert_eq!(channel.pending_count(), 1);

        // Verify the request has the right target
        let request = channel.pending.lock().pop_front();
        assert!(request.is_some());
        let request = request.expect("request should exist");
        assert!(matches!(request.target, CallableTarget::Closure(id) if id == closure_id));
    }

    #[test]
    fn test_callable_target_debug() {
        let func = CallableTarget::Function("test".to_string());
        let closure = CallableTarget::Closure(ClosureId(1));

        // Just verify Debug is implemented
        let _ = format!("{func:?}");
        let _ = format!("{closure:?}");
    }
}
