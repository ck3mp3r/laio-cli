use std::collections::HashMap;

use miette::Result;
use mockall::mock;

use super::client::HerdrApi;
use super::model::{
    LayoutApplyParams, LayoutDescription, LayoutNode, LayoutPane, PaneInfo, PaneSendInputParams,
    PaneZoomMode, PaneZoomResult, SessionSnapshot, WorkspaceCreateParams, WorkspaceCreated,
    WorkspaceInfo, WorkspaceReportMetadataParams,
};

use super::session_test_support::session_snapshot;
use super::support_models::*;

mock! {
    pub HerdrApiMock {}

    impl HerdrApi for HerdrApiMock {
        fn ping<'a>(&self, session: &'a str) -> Result<(String, u32)>;
        fn workspace_create<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b WorkspaceCreateParams,
        ) -> Result<WorkspaceCreated>;
        fn workspace_get<'a, 'b>(
            &self,
            session: &'a str,
            workspace_id: &'b str,
        ) -> Result<WorkspaceInfo>;
        fn workspace_focus<'a, 'b>(
            &self,
            session: &'a str,
            workspace_id: &'b str,
        ) -> Result<WorkspaceInfo>;
        fn workspace_close<'a, 'b>(
            &self,
            session: &'a str,
            workspace_id: &'b str,
        ) -> Result<()>;
        fn workspace_list<'a>(&self, session: &'a str) -> Result<Vec<WorkspaceInfo>>;
        fn session_snapshot<'a>(&self, session: &'a str) -> Result<SessionSnapshot>;
        fn workspace_report_metadata<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b WorkspaceReportMetadataParams,
        ) -> Result<()>;
        fn layout_apply<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b LayoutApplyParams,
        ) -> Result<LayoutDescription>;
        fn layout_export<'a, 'b>(
            &self,
            session: &'a str,
            tab_id: &'b str,
        ) -> Result<LayoutDescription>;
        fn pane_focus<'a, 'b>(&self, session: &'a str, pane_id: &'b str) -> Result<PaneInfo>;
        fn pane_zoom<'a, 'b>(
            &self,
            session: &'a str,
            pane_id: &'b str,
            mode: PaneZoomMode,
        ) -> Result<PaneZoomResult>;
        fn pane_send_input<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b PaneSendInputParams,
        ) -> Result<()>;
        fn session_exists<'a>(&self, session: &'a str) -> bool;
        fn ensure_server<'a>(&self, session: &'a str) -> Result<()>;
    }
}

#[test]
fn herdr_api_mock_covers_every_operation() -> Result<()> {
    let mut api = MockHerdrApiMock::new();

    api.expect_ping()
        .times(1)
        .returning(|_| Ok(("0.9.5".to_string(), 22)));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_get()
        .times(1)
        .returning(|_, _| Ok(workspace_info()));
    api.expect_workspace_focus()
        .times(1)
        .returning(|_, _| Ok(workspace_info()));
    api.expect_workspace_close()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info()]));
    api.expect_session_snapshot()
        .times(1)
        .returning(|_| Ok(session_snapshot()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .returning(|_, _| Ok(layout_description()));
    api.expect_layout_export()
        .times(1)
        .returning(|_, _| Ok(layout_description()));
    api.expect_pane_focus()
        .times(1)
        .returning(|_, _| Ok(pane_info()));
    api.expect_pane_zoom()
        .times(1)
        .returning(|_, _, _| Ok(pane_zoom_result()));
    api.expect_pane_send_input()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));

    let api: Box<dyn HerdrApi> = Box::new(api);

    assert_eq!(api.ping("w")?.1, 22);
    api.workspace_create(
        "w",
        &WorkspaceCreateParams {
            cwd: None,
            focus: false,
            label: None,
            env: HashMap::new(),
        },
    )?;
    api.workspace_get("w", "w1")?;
    api.workspace_focus("w", "w1")?;
    api.workspace_close("w", "w1")?;
    api.workspace_list("w")?;
    api.session_snapshot("w")?;
    api.workspace_report_metadata(
        "w",
        &WorkspaceReportMetadataParams {
            workspace_id: "w1".to_string(),
            source: "laio".to_string(),
            tokens: HashMap::new(),
            seq: None,
        },
    )?;
    api.layout_apply(
        "w",
        &LayoutApplyParams {
            workspace_id: None,
            tab_id: None,
            tab_label: None,
            focus: false,
            root: LayoutNode::Pane {
                pane: LayoutPane::default(),
            },
        },
    )?;
    api.layout_export("w", "t1")?;
    api.pane_focus("w", "p1")?;
    api.pane_zoom("w", "p1", PaneZoomMode::On)?;
    api.pane_send_input(
        "w",
        &PaneSendInputParams {
            pane_id: "p1".to_string(),
            text: "ls".to_string(),
            keys: Vec::new(),
        },
    )?;
    assert!(api.session_exists("w"));
    api.ensure_server("w")?;
    Ok(())
}
