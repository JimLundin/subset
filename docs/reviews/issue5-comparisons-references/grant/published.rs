use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The CLI's suggestion to allow calls like this one, made session-wide.
#[derive(Debug, Clone, PartialEq)]
struct Grant {
    label: String,
    update: Update,
}

/// An entry of `permission_suggestions` that allows calls by rule. Rules
/// are suggested for `localSettings`, which would outlive the session, so a
/// grant keeps only their rules, verbatim, and sends them for `session`.
/// Any other entry fails this parse and is skipped: a `setMode`, which
/// would allow every edit and change the mode behind the host's back, a
/// rule that denies or asks, and `addDirectories`.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Suggestion {
    AddRules { rules: Value, behavior: Behavior },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Behavior {
    Allow,
}

/// A permission rule, read only to name it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rule {
    tool_name: String,
    rule_content: Option<String>,
}

/// One entry of `updatedPermissions`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Update {
    AddRules {
        rules: Value,
        behavior: String,
        destination: String,
    },
}

impl Rule {
    /// The rule as the CLI writes it in settings: `Bash(git status)`.
    fn label(&self) -> String {
        match &self.rule_content {
            Some(content) => format!("{}({content})", self.tool_name),
            None => self.tool_name.clone(),
        }
    }
}

fn grant(suggestions: Option<Value>) -> Option<Grant> {
    let suggestions = suggestions
        .and_then(|value| serde_json::from_value::<Vec<Value>>(value).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| serde_json::from_value::<Suggestion>(value).ok())
        .collect::<Vec<Suggestion>>();
    let rules = suggestions
        .into_iter()
        .filter_map(|suggestion| match suggestion {
            Suggestion::AddRules {
                rules,
                behavior: Behavior::Allow,
            } => serde_json::from_value::<Vec<Value>>(rules).ok(),
        })
        .flatten()
        .collect::<Vec<Value>>();
    Some(rules)
        .filter(|rules| !rules.is_empty())
        .map(|rules| Grant {
            label: rules
                .iter()
                .filter_map(|rule| serde_json::from_value::<Rule>(rule.clone()).ok())
                .map(|rule| rule.label())
                .collect::<Vec<String>>()
                .join(", "),
            update: Update::AddRules {
                rules: Value::Array(rules),
                behavior: String::from("allow"),
                destination: String::from("session"),
            },
        })
}
