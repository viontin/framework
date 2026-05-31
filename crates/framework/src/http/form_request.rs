use crate::http::{Request, Response, StatusCode};

fn set_status(res: &mut Response, code: StatusCode) {
    res.status = code;
}

/// Form request validation and authorization.
pub trait FormRequest: std::fmt::Debug + Send + Sync {
    fn authorize(&self) -> bool { true }
    fn rules(&self) -> Vec<(&str, &str)> { Vec::new() }
    fn messages(&self) -> Vec<(&str, &str)> { Vec::new() }

    fn validate(&self, req: &Request) -> Result<(), Vec<String>> {
        let rules = self.rules();
        if rules.is_empty() { return Ok(()); }

        let body = req.body_str();
        let mut errors = Vec::new();

        for (field, rule_str) in &rules {
            let parts: Vec<&str> = rule_str.split('|').collect();
            let value = extract_field(body, field);

            for rule in &parts {
                match (*rule).splitn(2, ':').collect::<Vec<&str>>().as_slice() {
                    ["required"] => {
                        if value.is_none() || value == Some("") {
                            errors.push(format!("{} is required", field));
                        }
                    }
                    ["min", n] => {
                        if let Some(v) = value
                            && let Ok(min) = n.parse::<usize>()
                                && v.len() < min { errors.push(format!("{} must be at least {} characters", field, min)); }
                    }
                    ["email"] => {
                        if let Some(v) = value
                            && (!v.contains('@') || !v.contains('.')) { errors.push(format!("{} must be a valid email", field)); }
                    }
                    _ => {}
                }
            }
        }

        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    fn validate_or_reject(&self, req: &Request) -> Result<(), Response> {
        if !self.authorize() {
            let mut r = Response::html("Forbidden");
            set_status(&mut r, StatusCode::FORBIDDEN);
            return Err(r);
        }
        self.validate(req).map_err(|errors| {
            let body = serde_json::json!({ "errors": errors }).to_string();
            let mut r = Response::text(&body).header("content-type", "application/json");
            set_status(&mut r, StatusCode::BAD_REQUEST);
            r
        })
    }
}

fn extract_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    for pair in body.split('&') {
        let (key, val) = pair.split_once('=')?;
        
        
        if key == field { return Some(val); }
    }
    None
}
