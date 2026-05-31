use crate::project::identify_project;
use crate::types::{LightState, SessionRef, Status, Tool};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[derive(Default)]
struct AggregatorState {
    lights: HashMap<String, LightState>,
    session_to_project: HashMap<String, String>,
    light_order: Vec<String>,
}

#[derive(Clone, Default)]
pub struct StateAggregator {
    state: Arc<RwLock<AggregatorState>>,
    on_change: Arc<RwLock<Option<Arc<dyn Fn() + Send + Sync>>>>,
}

impl StateAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_session(&self, session_id: String, tool: Tool, cwd: &Path, status: Status) {
        {
            let (project_id, project_label) = identify_project(cwd);
            let mut state = self.state.write().expect("aggregator state lock poisoned");

            remove_existing_session(&mut state, &session_id);

            if !state.lights.contains_key(&project_id) {
                state.light_order.push(project_id.clone());
            }

            let light = state
                .lights
                .entry(project_id.clone())
                .or_insert_with(|| LightState::new(project_id.clone(), project_label));

            light.sessions.push(SessionRef {
                session_id: session_id.clone(),
                tool,
                status,
                started_at: Instant::now(),
            });
            light.last_event_at = Instant::now();
            light.aggregate_status();

            state.session_to_project.insert(session_id, project_id);
        }

        self.notify_change();
    }

    pub fn update_session_status(&self, session_id: &str, new_status: Status) {
        let mut changed = false;

        {
            let mut state = self.state.write().expect("aggregator state lock poisoned");
            let Some(project_id) = state.session_to_project.get(session_id).cloned() else {
                return;
            };

            if let Some(light) = state.lights.get_mut(&project_id) {
                if let Some(session) = light
                    .sessions
                    .iter_mut()
                    .find(|session| session.session_id == session_id)
                {
                    session.status = new_status;
                    light.last_event_at = Instant::now();
                    light.aggregate_status();
                    changed = true;
                }
            }
        }

        if changed {
            self.notify_change();
        }
    }

    pub fn session_status(&self, session_id: &str) -> Option<Status> {
        let state = self.state.read().expect("aggregator state lock poisoned");
        let project_id = state.session_to_project.get(session_id)?;
        let light = state.lights.get(project_id)?;

        light
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .map(|session| session.status)
    }

    pub fn remove_session(&self, session_id: &str) {
        let changed;

        {
            let mut state = self.state.write().expect("aggregator state lock poisoned");
            let Some(project_id) = state.session_to_project.remove(session_id) else {
                return;
            };

            let should_remove = if let Some(light) = state.lights.get_mut(&project_id) {
                light
                    .sessions
                    .retain(|session| session.session_id != session_id);
                light.last_event_at = Instant::now();

                if light.sessions.is_empty() {
                    true
                } else {
                    light.aggregate_status();
                    false
                }
            } else {
                false
            };

            if should_remove {
                remove_light_by_project(&mut state, &project_id);
            }
            changed = true;
        }

        if changed {
            self.notify_change();
        }
    }

    pub fn confirm_light(&self, project_id: &str) {
        let mut changed = false;

        {
            let mut state = self.state.write().expect("aggregator state lock poisoned");
            let Some(status) = state.lights.get(project_id).map(|light| light.status) else {
                return;
            };

            match status {
                Status::Done => {
                    remove_light_by_project(&mut state, project_id);
                    changed = true;
                }
                Status::Error => {
                    let Some(has_no_sessions) = state
                        .lights
                        .get(project_id)
                        .map(|light| light.sessions.is_empty())
                    else {
                        return;
                    };

                    if has_no_sessions {
                        remove_light_by_project(&mut state, project_id);
                        changed = true;
                    } else {
                        let Some(light) = state.lights.get_mut(project_id) else {
                            return;
                        };

                        for session in &mut light.sessions {
                            if session.status == Status::Error {
                                session.status = Status::Idle;
                                changed = true;
                            }
                        }
                        if changed {
                            light.last_event_at = Instant::now();
                            light.aggregate_status();
                        }
                    }
                }
                Status::Idle | Status::Working => {}
            }
        }

        if changed {
            self.notify_change();
        }
    }

    pub fn remove_light(&self, project_id: &str) {
        let removed = {
            let mut state = self.state.write().expect("aggregator state lock poisoned");
            remove_light_by_project(&mut state, project_id)
        };

        if removed {
            self.notify_change();
        }
    }

    pub fn get_lights(&self) -> Vec<LightState> {
        let state = self.state.read().expect("aggregator state lock poisoned");

        state
            .light_order
            .iter()
            .filter_map(|project_id| state.lights.get(project_id).cloned())
            .collect()
    }

    pub fn set_last_tool_call(&self, session_id: &str, tool_call: String) {
        let mut changed = false;

        {
            let mut state = self.state.write().expect("aggregator state lock poisoned");
            let Some(project_id) = state.session_to_project.get(session_id).cloned() else {
                return;
            };

            if let Some(light) = state.lights.get_mut(&project_id) {
                light.last_tool_call = Some(tool_call);
                light.last_event_at = Instant::now();
                changed = true;
            }
        }

        if changed {
            self.notify_change();
        }
    }

    pub fn set_on_change<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut on_change = self
            .on_change
            .write()
            .expect("aggregator callback lock poisoned");
        *on_change = Some(Arc::new(callback));
    }

    fn notify_change(&self) {
        let callback = self
            .on_change
            .read()
            .expect("aggregator callback lock poisoned")
            .clone();

        if let Some(callback) = callback {
            callback();
        }
    }
}

fn remove_existing_session(state: &mut AggregatorState, session_id: &str) {
    let Some(project_id) = state.session_to_project.remove(session_id) else {
        return;
    };

    let should_remove = if let Some(light) = state.lights.get_mut(&project_id) {
        light
            .sessions
            .retain(|session| session.session_id != session_id);

        if light.sessions.is_empty() {
            true
        } else {
            light.aggregate_status();
            false
        }
    } else {
        false
    };

    if should_remove {
        remove_light_by_project(state, &project_id);
    }
}

fn remove_light_by_project(state: &mut AggregatorState, project_id: &str) -> bool {
    let mut removed = false;

    if let Some(light) = state.lights.remove(project_id) {
        for session in light.sessions {
            state.session_to_project.remove(&session.session_id);
        }
        removed = true;
    }

    state
        .light_order
        .retain(|existing_project_id| existing_project_id != project_id);

    removed
}
