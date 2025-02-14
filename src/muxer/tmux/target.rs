use std::fmt;

pub(crate) struct Target {
    pub session: String,
    pub window: Option<String>,
    pub pane: Option<String>,
}

#[macro_export]
macro_rules! tmux_target {
    ($session:expr) => {
        Target::new($session)
    };
    ($session:expr, $window:expr) => {
        Target::new($session).window($window)
    };
    ($session:expr, $window:expr, $pane:expr) => {
        Target::new($session).window($window).pane($pane)
    };
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut target = String::new();

        target.push_str(self.session.to_string().as_str());

        if let Some(window) = &self.window {
            if !target.is_empty() {
                target.push(':');
            }
            target.push_str(window);
        }

        if let Some(pane) = &self.pane {
            if !target.is_empty() {
                target.push('.');
            }
            target.push_str(pane);
        }

        write!(f, "{}", target)
    }
}

impl Target {
    pub fn new(session: &str) -> Self {
        Target {
            session: session.to_string(),
            window: None,
            pane: None,
        }
    }

    pub fn window(mut self, window: &str) -> Self {
        self.window = Some(window.to_string());
        self
    }

    pub fn pane(mut self, pane: &str) -> Self {
        self.pane = Some(pane.to_string());
        self
    }
}
