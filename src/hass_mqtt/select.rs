use crate::hass_mqtt::base::{Device, EntityConfig, Origin};
use crate::hass_mqtt::instance::{publish_entity_config, EntityInstance};
use crate::hass_mqtt::work_mode::ParsedWorkMode;
use crate::service::device::Device as ServiceDevice;
use crate::service::hass::{availability_topic, topic_safe_id, HassClient};
use crate::service::state::StateHandle;
use axum::async_trait;
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Clone, Debug)]
pub struct SelectConfig {
    #[serde(flatten)]
    pub base: EntityConfig,

    pub command_topic: String,
    pub options: Vec<String>,
    pub state_topic: String,
}

impl SelectConfig {
    pub async fn publish(&self, state: &StateHandle, client: &HassClient) -> anyhow::Result<()> {
        publish_entity_config("select", state, client, &self.base, self).await
    }
}

pub struct WorkModeSelect {
    select: SelectConfig,
    device_id: String,
    state: StateHandle,
}

impl WorkModeSelect {
    pub fn new(device: &ServiceDevice, work_modes: &ParsedWorkMode, state: &StateHandle) -> Self {
        let command_topic = format!("gv2mqtt/{id}/set-work-mode", id = topic_safe_id(device),);
        let state_topic = format!("gv2mqtt/{id}/notify-work-mode", id = topic_safe_id(device));
        let availability_topic = availability_topic();
        let unique_id = format!("gv2mqtt-{id}-workMode", id = topic_safe_id(device),);

        Self {
            select: SelectConfig {
                base: EntityConfig {
                    availability_topic,
                    name: Some("Mode".to_string()),
                    device_class: None,
                    origin: Origin::default(),
                    device: Device::for_device(device),
                    unique_id,
                    entity_category: None,
                    icon: None,
                },
                command_topic,
                state_topic,
                options: work_modes.get_mode_names(),
            },
            device_id: device.id.to_string(),
            state: state.clone(),
        }
    }
}

#[async_trait]
impl EntityInstance for WorkModeSelect {
    async fn publish_config(&self, state: &StateHandle, client: &HassClient) -> anyhow::Result<()> {
        self.select.publish(&state, &client).await
    }

    async fn notify_state(&self, client: &HassClient) -> anyhow::Result<()> {
        let device = self
            .state
            .device_by_id(&self.device_id)
            .await
            .expect("device to exist");

        if let Some(mode_value) = device.humidifier_work_mode {
            if let Ok(work_mode) = ParsedWorkMode::with_device(&device) {
                let mode_value_json = json!(mode_value);
                if let Some(mode) = work_mode.mode_for_value(&mode_value_json) {
                    client
                        .publish(&self.select.state_topic, mode.name.to_string())
                        .await?;
                }
            }
        } else {
            let work_modes = ParsedWorkMode::with_device(&device)?;

            if let Some(state) = &device.http_device_state {
                for cap in &state.capabilities {
                    if cap.instance == "workMode" {
                        if let Some(mode_num) = cap.state.pointer("/value/workMode") {
                            if let Some(mode) = work_modes.mode_for_value(mode_num) {
                                return client
                                    .publish(&self.select.state_topic, mode.name.to_string())
                                    .await;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
