use std::fmt::Display;

use regex::Regex;
use serde::{Deserialize, Serialize};
use settings::macros::{define_settings_group, maybe_define_setting, register_settings_events};
use settings::{Setting, SupportedPlatforms};
use warp_errors::report_error;
pub use warp_terminal::model::secrets::RegexDisplayInfo;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

pub const TELEMETRY_ENABLED_DEFAULTS_KEY: &str = "TelemetryEnabled";
pub const CRASH_REPORTING_ENABLED_DEFAULTS_KEY: &str = "CrashReportingEnabled";

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "A custom regex pattern for detecting and redacting secrets.")]
pub struct CustomSecretRegex {
    #[serde(with = "serde_regex")]
    #[schemars(with = "String", description = "The regex pattern to match secrets.")]
    pub pattern: Regex,
    #[serde(default)]
    #[schemars(description = "Optional display name for this secret pattern.")]
    pub name: Option<String>,
}

impl CustomSecretRegex {
    pub fn pattern(&self) -> &Regex {
        &self.pattern
    }
}

impl RegexDisplayInfo for CustomSecretRegex {
    fn pattern(&self) -> &str {
        self.pattern.as_str()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl Display for CustomSecretRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pattern.as_str())
    }
}

impl PartialEq for CustomSecretRegex {
    /// We do not factor in the name to equality checks --
    /// if the regex is the same, then the regex is the same.
    /// This allows us to avoid adding duplicate regexes.
    fn eq(&self, other: &Self) -> bool {
        self.pattern.as_str() == other.pattern.as_str()
    }
}

impl settings_value::SettingsValue for CustomSecretRegex {}

define_settings_group!(LocalPrivacySettings, settings: [
    is_telemetry_enabled: IsTelemetryEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        private: false,
        storage_key: "TelemetryEnabled",
        toml_path: "privacy.telemetry_enabled",
        description: "Whether anonymous usage telemetry is collected.",
    },
    is_crash_reporting_enabled: IsCrashReportingEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        private: false,
        storage_key: "CrashReportingEnabled",
        toml_path: "privacy.crash_reporting_enabled",
        description: "Whether crash reports are sent.",
    },

]);

maybe_define_setting!(CustomSecretRegexList, group: PrivacySettings, {
    type: Vec<CustomSecretRegex>,
    default: Vec::new(),
    supported_platforms: SupportedPlatforms::ALL,
    private: false,
    toml_path: "privacy.custom_secret_regex_list",
    description: "Custom regex patterns for detecting and redacting secrets.",
});

maybe_define_setting!(HasInitializedDefaultSecretRegexes, group: PrivacySettings, {
    type: bool,
    default: false,
    supported_platforms: SupportedPlatforms::ALL,
    private: true,
});

/// Singleton model for managing the user's privacy settings (whether the user has enabled crash
/// reporting and/or telemetry).
pub struct PrivacySettings {
    pub is_telemetry_enabled: bool,
    pub is_crash_reporting_enabled: bool,

    pub has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes,
    pub user_secret_regex_list: CustomSecretRegexList,
}

/// A snapshot of a user's [`PrivacySettings`] settings at some point in time.
#[derive(Clone, Copy)]
pub struct PrivacySettingsSnapshot {
    is_telemetry_enabled: bool,
    is_crash_reporting_enabled: bool,
}

impl PrivacySettingsSnapshot {
    pub fn is_telemetry_enabled(&self) -> bool {
        self.is_telemetry_enabled
    }

    pub fn is_crash_reporting_enabled(&self) -> bool {
        self.is_crash_reporting_enabled
    }

    pub fn should_disable_telemetry(&self) -> bool {
        !self.is_telemetry_enabled
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            is_telemetry_enabled: true,
            is_crash_reporting_enabled: true,
        }
    }
}

impl PrivacySettings {
    /// Registers a singleton PrivacySettings model on `app`.
    ///
    /// We expose this function publicly (while keeping the constructor private) to prevent
    /// instantiation another PrivacySettings struct, in the case where a developer might be
    /// unaware that it is registered as a singleton model.
    pub fn register_singleton(ctx: &mut AppContext) {
        let handle = ctx.add_singleton_model(PrivacySettings::new);

        register_settings_events!(
            PrivacySettings,
            user_secret_regex_list,
            CustomSecretRegexList,
            handle,
            ctx
        );
    }

    /// Initializes privacy settings from local storage.
    fn new(ctx: &mut ModelContext<Self>) -> Self {
        // Initialize from `LocalPrivacySettings`, which is the source of truth for these
        // booleans.
        let local_privacy = LocalPrivacySettings::as_ref(ctx);
        let is_telemetry_enabled = *local_privacy.is_telemetry_enabled.value();
        let is_crash_reporting_enabled = *local_privacy.is_crash_reporting_enabled.value();
        ctx.subscribe_to_model(&LocalPrivacySettings::handle(ctx), |me, _, event, ctx| {
            let privacy_settings = LocalPrivacySettings::as_ref(ctx);
            match event {
                LocalPrivacySettingsChangedEvent::IsTelemetryEnabled { .. } => {
                    me.set_is_telemetry_enabled(
                        *privacy_settings.is_telemetry_enabled.value(),
                        ctx,
                    );
                }
                LocalPrivacySettingsChangedEvent::IsCrashReportingEnabled { .. } => {
                    me.set_is_crash_reporting_enabled(
                        *privacy_settings.is_crash_reporting_enabled.value(),
                        ctx,
                    );
                }
            }
        });

        let user_secret_regex_list: CustomSecretRegexList =
            CustomSecretRegexList::new_from_storage(ctx);
        let has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes =
            HasInitializedDefaultSecretRegexes::new_from_storage(ctx);

        Self {
            is_crash_reporting_enabled,
            is_telemetry_enabled,

            user_secret_regex_list,
            has_initialized_default_secret_regexes,
        }
    }

    /// Constructor for tests only.
    #[cfg(any(test, feature = "test-util"))]
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            is_crash_reporting_enabled: true,
            is_telemetry_enabled: true,

            user_secret_regex_list: CustomSecretRegexList::new(None),
            has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes::new(None),
        }
    }

    /// Returns a snapshot of the user's privacy settings.
    ///
    /// The returned snapshot is not stateful, thus its values should be used shortly after the
    /// snapshot is returned.
    pub fn get_snapshot(&self) -> PrivacySettingsSnapshot {
        PrivacySettingsSnapshot {
            is_telemetry_enabled: self.is_telemetry_enabled,
            is_crash_reporting_enabled: self.is_crash_reporting_enabled,
        }
    }

    /// Sets `is_crash_reporting_enabled` to the given value.
    ///
    /// Persists the value in local settings.
    /// Finally, emits a `PrivacySettingsEvent::UpdateIsCrashReportingEnabled` event.
    pub fn set_is_crash_reporting_enabled(
        &mut self,
        new_value: bool,
        ctx: &mut ModelContext<PrivacySettings>,
    ) {
        let old_value = self.is_crash_reporting_enabled;
        if new_value != old_value {
            self.is_crash_reporting_enabled = new_value;

            LocalPrivacySettings::handle(ctx).update(ctx, |settings, ctx| {
                log::info!("Setting is_crash_reporting_enabled to {new_value}");
                let _ = settings
                    .is_crash_reporting_enabled
                    .set_value(new_value, ctx);
            });

            ctx.emit(PrivacySettingsChangedEvent::UpdateIsCrashReportingEnabled {
                old_value,
                new_value,
            });
            ctx.notify();
        }
    }

    /// Sets `is_telemetry_enabled` to the given value.
    ///
    /// Persists the value in local settings.
    /// Finally, emits a `PrivacySettingsEvent::UpdateIsTelemetryEnabled` event.
    pub fn set_is_telemetry_enabled(
        &mut self,
        new_value: bool,
        ctx: &mut ModelContext<PrivacySettings>,
    ) {
        let old_value = self.is_telemetry_enabled;
        if new_value != old_value {
            self.is_telemetry_enabled = new_value;

            LocalPrivacySettings::handle(ctx).update(ctx, |settings, ctx| {
                log::info!("Setting is_telemetry_enabled to {new_value}");
                let _ = settings.is_telemetry_enabled.set_value(new_value, ctx);
            });

            ctx.emit(PrivacySettingsChangedEvent::UpdateIsTelemetryEnabled {
                old_value,
                new_value,
            });
            ctx.notify();
        }
    }

    pub fn remove_user_secret_regex(&mut self, idx: &usize, ctx: &mut ModelContext<Self>) {
        let mut new_user_secret_regex_list = self.user_secret_regex_list.to_vec();
        new_user_secret_regex_list.remove(*idx);
        if self
            .user_secret_regex_list
            .set_value(new_user_secret_regex_list, ctx)
            .is_err()
        {
            report_error!("Custom Secret Regex List failed to serialize")
        }
    }

    /// Initializes the custom secret regex list with the default regexes, provided
    /// non matches can be found.
    /// This can be called when a user first enables secret redaction.
    pub fn add_all_recommended_regex(&mut self, ctx: &mut ModelContext<Self>) {
        let mut new_user_secret_regex_list = self.user_secret_regex_list.to_vec();
        let num_existing_regexes = new_user_secret_regex_list.len();

        // Add all the default regexes if they don't already exist
        for default_regex in crate::terminal::model::secrets::regexes::DEFAULT_REGEXES_WITH_NAMES {
            match Regex::new(default_regex.pattern) {
                Ok(regex) => {
                    let custom_regex = CustomSecretRegex {
                        pattern: regex,
                        name: Some(default_regex.name.to_string()),
                    };
                    if !new_user_secret_regex_list.contains(&custom_regex) {
                        new_user_secret_regex_list.push(custom_regex);
                    }
                }
                _ => {
                    report_error!(
                        "Failed to compile default regex",
                        extra: { "pattern" => %default_regex.pattern }
                    );
                }
            }
        }

        if num_existing_regexes == new_user_secret_regex_list.len() {
            return;
        }

        if self
            .user_secret_regex_list
            .set_value(new_user_secret_regex_list, ctx)
            .is_err()
        {
            report_error!("Failed to serialize default regexes to custom secret regex list")
        }

        ctx.notify();
    }

    /// Disables the default regex trigger, so that it will not be executed.
    pub fn disable_default_regex_trigger(&mut self, ctx: &mut ModelContext<Self>) {
        if self
            .has_initialized_default_secret_regexes
            .set_value(true, ctx)
            .is_err()
        {
            report_error!("Failed to disable default regex trigger");
        }
    }

    /// Initializes the custom secret regex list with the default regexes.
    /// This will only be executed once per user, and only if they haven't already initialized.
    pub fn initialize_default_regexes_once(&mut self, ctx: &mut ModelContext<Self>) {
        // Only initialize if we haven't done so before
        if !*self.has_initialized_default_secret_regexes.value() {
            self.add_all_recommended_regex(ctx);

            // Mark as initialized
            if self
                .has_initialized_default_secret_regexes
                .set_value(true, ctx)
                .is_err()
            {
                report_error!("Failed to set has_initialized_default_secret_regexes flag");
            }
        }
    }
}

/// Events emitted when PrivacySettings is updated.
#[derive(Clone, Copy)]
pub enum PrivacySettingsChangedEvent {
    UpdateIsTelemetryEnabled {
        old_value: bool,
        new_value: bool,
    },
    UpdateIsCrashReportingEnabled {
        old_value: bool,
        new_value: bool,
    },

    CustomSecretRegexList {
        change_event_reason: ChangeEventReason,
    },
    HasInitializedDefaultSecretRegexes {
        change_event_reason: ChangeEventReason,
    },
}

impl Entity for PrivacySettings {
    type Event = PrivacySettingsChangedEvent;
}

impl SingletonEntity for PrivacySettings {}
