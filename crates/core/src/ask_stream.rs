//! Bounded process-local Ask runs. Cancellation closes the inference socket;
//! stopping UI polling alone never counts as cancelling backend work.
use crate::{
    CommandError, Core, ErrorCode,
    ask::{AskRequest, AskResponse},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{Shutdown, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

#[derive(Default)]
pub(crate) struct Control {
    pub cancelled: AtomicBool,
    pub finished: AtomicBool,
    pub answer: Mutex<String>,
    pub socket: Mutex<Option<TcpStream>>,
}
impl Control {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(socket) = self.socket.lock()
            && let Some(socket) = socket.as_ref()
        {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}
pub(crate) struct Run {
    workspace: String,
    created: Instant,
    control: Arc<Control>,
    result: Option<Result<AskResponse, CommandError>>,
}
pub(crate) type Runs = Arc<Mutex<HashMap<String, Run>>>;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AskRunRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AskRunResponse {
    pub contract_version: u8,
    pub id: String,
    pub state: String,
    pub partial_answer: String,
    pub response: Option<AskResponse>,
    pub error: Option<CommandError>,
}

impl Core {
    pub fn start_ask(&self, request: AskRequest) -> Result<AskRunResponse, CommandError> {
        if request.contract_version != 1
            || request.prompt.trim().is_empty()
            || request.prompt.len() > 500
        {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Use a bounded Ask request.",
            ));
        }
        self.workspace_command(
            "open",
            serde_json::json!({"contractVersion":1,"id":request.workspace_id}),
        )?;
        let id = crate::content::random_id()?;
        let control = Arc::new(Control::default());
        let mut runs = self
            .ask_runs
            .lock()
            .map_err(|_| crate::storage::unavailable())?;
        // Results are transient. Bound retained results and actively cancel an
        // expired run before dropping its capability.
        runs.retain(|_, run| {
            if run.created.elapsed().as_secs() > 600 {
                run.control.cancel();
                false
            } else {
                true
            }
        });
        if runs.len() >= 4 {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "At most four local Ask runs may be retained. Cancel or forget a run first.",
            ));
        }
        runs.insert(
            id.clone(),
            Run {
                workspace: request.workspace_id.clone(),
                created: Instant::now(),
                control: control.clone(),
                result: None,
            },
        );
        drop(runs);
        let core = self.clone();
        let run_id = id.clone();
        std::thread::spawn(move || {
            let response = core.ask_anansi_controlled(request, Some(&control));
            if let Ok(mut runs) = core.ask_runs.lock()
                && let Some(run) = runs.get_mut(&run_id)
            {
                run.result = Some(response);
            }
        });
        Ok(AskRunResponse {
            contract_version: 1,
            id,
            state: "running".into(),
            partial_answer: String::new(),
            response: None,
            error: None,
        })
    }

    pub fn control_ask(
        &self,
        operation: &str,
        request: AskRunRequest,
    ) -> Result<AskRunResponse, CommandError> {
        if request.contract_version != 1 {
            return Err(CommandError::new(
                ErrorCode::UnsupportedContract,
                "Unsupported Ask contract.",
            ));
        }
        let mut runs = self
            .ask_runs
            .lock()
            .map_err(|_| crate::storage::unavailable())?;
        let run = runs
            .get_mut(&request.id)
            .filter(|run| run.workspace == request.workspace_id)
            .ok_or_else(|| {
                CommandError::new(ErrorCode::NotFound, "Ask run not found in this workspace.")
            })?;
        if operation == "ask-cancel" {
            run.control.cancel();
        }
        if operation == "ask-forget" {
            run.control.cancel();
        }
        let cancelled = run.control.cancelled.load(Ordering::SeqCst);
        let (response, error) = match &run.result {
            Some(Ok(response)) if !cancelled => (Some(response.clone()), None),
            Some(Err(error)) if !cancelled => (None, Some(error.clone())),
            _ => (None, None),
        };
        let partial_answer = if cancelled {
            String::new()
        } else {
            run.control
                .answer
                .lock()
                .map_err(|_| crate::storage::unavailable())?
                .clone()
        };
        let state = if cancelled {
            "cancelled"
        } else if run.result.is_some() {
            "completed"
        } else {
            "running"
        };
        let result = AskRunResponse {
            contract_version: 1,
            id: request.id.clone(),
            state: state.into(),
            partial_answer,
            response,
            error,
        };
        if operation == "ask-forget" {
            runs.remove(&request.id);
        }
        Ok(result)
    }
}
