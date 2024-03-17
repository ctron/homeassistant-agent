use crate::model::Component;
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct DeviceId {
    pub id: Cow<'static, str>,
    pub component: Component,
    pub node_id: Option<Cow<'static, str>>,
}

impl DeviceId {
    pub fn new(id: impl Into<Cow<'static, str>>, component: Component) -> Self {
        Self {
            id: id.into(),
            component,
            node_id: None,
        }
    }
    pub fn with_node_id<I, C>(
        id: impl Into<Cow<'static, str>>,
        component: Component,
        node_id: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            id: id.into(),
            component,
            node_id: Some(node_id.into()),
        }
    }

    /// render the config topic
    pub fn config_topic(&self) -> String {
        format!(
            "{component}/{node_id}{node_id_slash}{object_id}/config",
            component = self.component,
            object_id = self.id,
            node_id_slash = if self.node_id.is_some() { "/" } else { "" },
            node_id = self.node_id.as_deref().unwrap_or(""),
        )
    }
}
