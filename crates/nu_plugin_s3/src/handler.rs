use nu_errors::ShellError;
use nu_protocol::{CallInfo, CommandAction, ReturnSuccess, ReturnValue, UntaggedValue, Value};
use nu_source::{AnchorLocation, Tag};
use s3handler::{CredentialConfig, Handler as S3Handler};

pub struct Handler {
    pub resource: Option<Value>,
    pub tag: Tag,
    pub has_raw: bool,
    pub config: CredentialConfig,
}

impl Handler {
    pub fn new() -> Handler {
        Handler {
            tag: Tag::unknown(),
            config: CredentialConfig {
                host: String::new(),
                access_key: String::new(),
                secret_key: String::new(),
                user: None,
                region: None,
                s3_type: None,
                secure: None,
            },
            resource: None,
            has_raw: false,
        }
    }

    pub fn setup(&mut self, call_info: CallInfo) -> ReturnValue {
        self.resource = {
            let r = call_info.args.nth(0).ok_or_else(|| {
                ShellError::labeled_error(
                    "No obj or directory specified",
                    "for command",
                    &call_info.name_tag,
                )
            })?;
            Some(r.clone())
        };
        self.tag = call_info.name_tag.clone();
        self.has_raw = call_info.args.has("raw");

        if let Some(e) = call_info.args.get("endpoint") {
            self.config.host = e.as_string()?
        } else {
            return Err(ShellError::labeled_error(
                "No endpoint provided",
                "for command",
                &call_info.name_tag,
            ));
        }

        if let Some(access_key) = call_info.args.get("access-key") {
            self.config.access_key = access_key.as_string()?
        } else {
            return Err(ShellError::labeled_error(
                "No access key provided",
                "for command",
                &call_info.name_tag,
            ));
        }

        if let Some(secret_key) = call_info.args.get("secret-key") {
            self.config.secret_key = secret_key.as_string()?
        } else {
            return Err(ShellError::labeled_error(
                "No secret key provided",
                "for command",
                &call_info.name_tag,
            ));
        }

        if let Some(region) = call_info.args.get("region") {
            self.config.region = Some(region.as_string()?)
        }

        ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value())
    }
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn s3_helper(resource: &Value, has_raw: bool, config: &CredentialConfig) -> ReturnValue {
    let resource_str = resource.as_string()?;
    let mut handler = S3Handler::from(config);
    let (output, content_type) = handler
        .cat(&resource_str)
        .map_err(|e| ShellError::unexpected(e.to_string()))?;

    let extension = if has_raw {
        None
    } else {
        fn get_accept_ext(s: String) -> Option<String> {
            if s.contains("json") {
                Some("json".to_string())
            } else if s.contains("xml") {
                Some("xml".to_string())
            } else if s.contains("svg") {
                Some("svg".to_string())
            } else if s.contains("html") {
                Some("html".to_string())
            } else {
                None
            }
        }
        // If the extension could not provide when uploading,
        // try to use the resource extension.
        content_type.and_then(get_accept_ext).or_else(|| {
            resource_str
                .split('.')
                .last()
                .map(String::from)
                .and_then(get_accept_ext)
        })
    };

    if let Some(e) = extension {
        Ok(ReturnSuccess::Action(CommandAction::AutoConvert(
            UntaggedValue::string(output).into_value(Tag {
                span: resource.tag.span,
                anchor: Some(AnchorLocation::Url(resource_str)),
            }),
            e,
        )))
    } else {
        ReturnSuccess::value(UntaggedValue::string(output))
    }
}
