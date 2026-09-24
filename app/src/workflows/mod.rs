pub use cloud_object_models::WorkflowId;
use serde::{Deserialize, Serialize};
use warpui::AppContext;

pub mod categories;
use workflow::Workflow;

pub mod arguments;
pub mod command_parser;
pub mod info_box;
pub mod local_workflows;
pub mod workflow;

pub use categories::{CategoriesView, CategoriesViewEvent, WorkflowsViewAction};

use crate::notebooks::{NotebookId, NotebookLocation};
use crate::server::ids::ServerId;

pub fn init(app: &mut AppContext) {
    categories::init(app);
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub enum WorkflowSource {
    Global,
    Local,
    Project,
    Team {
        team_uid: ServerId,
    },
    PersonalCloud,
    WarpAI,
    Notebook {
        notebook_id: Option<NotebookId>,
        team_uid: Option<ServerId>,
        location: NotebookLocation,
    },

    /// A hardcoded workflow type that allows Warp to surface features as Workflows (e.g.
    /// a command to see our network log)
    App,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, PartialOrd)]
pub enum WorkflowSelectionSource {
    WarpDrive,
    CommandPalette,
    UniversalSearch,
    Voltron,
    WarpAI,
    Notebook,
    SlashMenu,
    UpArrowHistory,
    WorkflowView,
    AgentMode,
    Undefined,
    Alias,
}

/// A saved workflow or a command in a local notebook.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowType {
    /// Saved workflows sourced from local, global, project, app collections, saved locally.
    Local(Workflow),
    /// A command in a local notebook.
    Notebook(Workflow),
}

impl WorkflowType {
    pub fn as_workflow(&self) -> &Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }

    /// Returns the contained [`Workflow`], consuming `self`.
    pub fn take_workflow(self) -> Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }
}
