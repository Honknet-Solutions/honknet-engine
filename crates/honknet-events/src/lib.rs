use honknet_core::Entity;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Broadcast,
    Entity(Entity),
}

pub struct EventEnvelope {
    pub delivery: Delivery,
    pub predicted: bool,
    pub cancelled: bool,
    payload: Box<dyn Any + Send>,
}

impl EventEnvelope {
    pub fn new<T: Any + Send>(delivery: Delivery, predicted: bool, value: T) -> Self {
        Self {
            delivery,
            predicted,
            cancelled: false,
            payload: Box::new(value),
        }
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.payload.downcast_ref()
    }
}

#[derive(Default, Clone)]
pub struct EventBus {
    queues: Arc<Mutex<HashMap<TypeId, Vec<EventEnvelope>>>>,
}

impl EventBus {
    pub fn send<T: Any + Send>(&self, delivery: Delivery, predicted: bool, value: T) {
        self.queues
            .lock()
            .entry(TypeId::of::<T>())
            .or_default()
            .push(EventEnvelope::new(delivery, predicted, value));
    }
    pub fn drain<T: Any + Send>(&self) -> Vec<EventEnvelope> {
        self.queues
            .lock()
            .remove(&TypeId::of::<T>())
            .unwrap_or_default()
    }
    pub fn len<T: Any>(&self) -> usize {
        self.queues
            .lock()
            .get(&TypeId::of::<T>())
            .map_or(0, Vec::len)
    }
    pub fn is_empty<T: Any>(&self) -> bool {
        self.len::<T>() == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SignalId(String);

impl SignalId {
    pub fn new(value: impl Into<String>) -> Result<Self, SignalError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic);
        if valid {
            Ok(Self(value))
        } else {
            Err(SignalError::InvalidId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SignalTarget {
    Global,
    Entity { entity: Entity },
    Component { entity: Entity, component: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalContext {
    pub id: SignalId,
    pub target: SignalTarget,
    pub payload: serde_json::Value,
    pub cancellable: bool,
    pub cancelled: bool,
    pub propagation_stopped: bool,
}

impl SignalContext {
    pub fn new(
        id: SignalId,
        target: SignalTarget,
        payload: serde_json::Value,
        cancellable: bool,
    ) -> Self {
        Self {
            id,
            target,
            payload,
            cancellable,
            cancelled: false,
            propagation_stopped: false,
        }
    }

    pub fn cancel(&mut self) -> bool {
        if self.cancellable {
            self.cancelled = true;
            true
        } else {
            false
        }
    }

    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalTargetFilter {
    Any,
    Global,
    Entity(Entity),
    Component {
        entity: Option<Entity>,
        component: String,
    },
}

impl SignalTargetFilter {
    fn matches(&self, target: &SignalTarget) -> bool {
        match (self, target) {
            (Self::Any, _) | (Self::Global, SignalTarget::Global) => true,
            (Self::Entity(expected), SignalTarget::Entity { entity }) => expected == entity,
            (
                Self::Component {
                    entity: expected_entity,
                    component: expected_component,
                },
                SignalTarget::Component { entity, component },
            ) => {
                expected_entity.is_none_or(|expected| expected == *entity)
                    && expected_component == component
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionToken(u64);

type SignalHandler = Arc<dyn Fn(&mut SignalContext) + Send + Sync>;

struct SignalSubscription {
    token: SubscriptionToken,
    signal: SignalId,
    target: SignalTargetFilter,
    priority: i32,
    sequence: u64,
    handler: SignalHandler,
}

#[derive(Default)]
struct SignalBusState {
    next_token: u64,
    next_sequence: u64,
    subscriptions: Vec<SignalSubscription>,
}

#[derive(Default, Clone)]
pub struct SignalBus {
    state: Arc<Mutex<SignalBusState>>,
}

impl SignalBus {
    pub fn subscribe<F>(
        &self,
        signal: SignalId,
        target: SignalTargetFilter,
        priority: i32,
        handler: F,
    ) -> SubscriptionToken
    where
        F: Fn(&mut SignalContext) + Send + Sync + 'static,
    {
        let mut state = self.state.lock();
        let token = SubscriptionToken(state.next_token);
        state.next_token = state.next_token.wrapping_add(1);
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.wrapping_add(1);
        state.subscriptions.push(SignalSubscription {
            token,
            signal,
            target,
            priority,
            sequence,
            handler: Arc::new(handler),
        });
        token
    }

    pub fn unsubscribe(&self, token: SubscriptionToken) -> bool {
        let mut state = self.state.lock();
        let before = state.subscriptions.len();
        state
            .subscriptions
            .retain(|subscription| subscription.token != token);
        state.subscriptions.len() != before
    }

    pub fn emit(&self, context: &mut SignalContext) {
        let mut handlers: Vec<_> = self
            .state
            .lock()
            .subscriptions
            .iter()
            .filter(|subscription| {
                subscription.signal == context.id && subscription.target.matches(&context.target)
            })
            .map(|subscription| {
                (
                    subscription.priority,
                    subscription.sequence,
                    Arc::clone(&subscription.handler),
                )
            })
            .collect();
        handlers.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));

        for (_, _, handler) in handlers {
            handler(context);
            if context.propagation_stopped {
                break;
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("invalid signal ID: {0}")]
    InvalidId(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_handlers_run_by_priority_then_registration_order() {
        let bus = SignalBus::default();
        let id = SignalId::new("game.damageAttempt").unwrap();
        bus.subscribe(id.clone(), SignalTargetFilter::Any, 0, |context| {
            context.payload["order"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!("normal"));
        });
        bus.subscribe(id.clone(), SignalTargetFilter::Any, 100, |context| {
            context.payload["order"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!("first"));
        });
        let mut context = SignalContext::new(
            id,
            SignalTarget::Global,
            serde_json::json!({ "order": [] }),
            true,
        );

        bus.emit(&mut context);

        assert_eq!(
            context.payload["order"],
            serde_json::json!(["first", "normal"])
        );
    }

    #[test]
    fn cancellation_and_propagation_are_independent() {
        let bus = SignalBus::default();
        let id = SignalId::new("game.interactionAttempt").unwrap();
        bus.subscribe(id.clone(), SignalTargetFilter::Global, 10, |context| {
            context.cancel();
            context.stop_propagation();
        });
        bus.subscribe(id.clone(), SignalTargetFilter::Any, 0, |context| {
            context.payload = serde_json::json!("must not run");
        });
        let mut context =
            SignalContext::new(id, SignalTarget::Global, serde_json::Value::Null, true);

        bus.emit(&mut context);

        assert!(context.cancelled);
        assert!(context.propagation_stopped);
        assert_eq!(context.payload, serde_json::Value::Null);
    }

    #[test]
    fn non_cancellable_signal_cannot_be_cancelled() {
        let mut context = SignalContext::new(
            SignalId::new("game.tick").unwrap(),
            SignalTarget::Global,
            serde_json::Value::Null,
            false,
        );
        assert!(!context.cancel());
        assert!(!context.cancelled);
    }
}
