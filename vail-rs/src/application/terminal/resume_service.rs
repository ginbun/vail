use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct ResumeBinding {
    pub user_id: i64,
    pub host_id: i64,
    pub connect_type: String,
}

#[derive(Debug, Clone)]
pub struct ResumeRequest<'a> {
    pub user_id: i64,
    pub host_id: i64,
    pub connect_type: &'a str,
}

pub fn ensure_resume_binding(binding: &ResumeBinding, request: ResumeRequest<'_>) -> AppResult<()> {
    if binding.user_id != request.user_id
        || binding.host_id != request.host_id
        || !binding.connect_type.eq_ignore_ascii_case(request.connect_type)
    {
        return Err(AppError::Auth(
            "resume session ownership or target mismatch".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_resume_binding_rejects_cross_user_reattach() {
        let binding = ResumeBinding {
            user_id: 100,
            host_id: 200,
            connect_type: "ssh".to_string(),
        };
        let err = ensure_resume_binding(
            &binding,
            ResumeRequest {
                user_id: 101,
                host_id: 200,
                connect_type: "ssh",
            },
        )
        .expect_err("must reject");
        assert!(err.to_string().contains("mismatch"));
    }

    #[test]
    fn ensure_resume_binding_rejects_cross_host_reattach() {
        let binding = ResumeBinding {
            user_id: 100,
            host_id: 200,
            connect_type: "ssh".to_string(),
        };
        let err = ensure_resume_binding(
            &binding,
            ResumeRequest {
                user_id: 100,
                host_id: 201,
                connect_type: "ssh",
            },
        )
        .expect_err("must reject");
        assert!(err.to_string().contains("mismatch"));
    }

    #[test]
    fn ensure_resume_binding_rejects_connect_type_mismatch() {
        let binding = ResumeBinding {
            user_id: 100,
            host_id: 200,
            connect_type: "ssh".to_string(),
        };
        let err = ensure_resume_binding(
            &binding,
            ResumeRequest {
                user_id: 100,
                host_id: 200,
                connect_type: "sftp",
            },
        )
        .expect_err("must reject");
        assert!(err.to_string().contains("mismatch"));
    }

    #[test]
    fn ensure_resume_binding_accepts_same_context() {
        let binding = ResumeBinding {
            user_id: 100,
            host_id: 200,
            connect_type: "ssh".to_string(),
        };
        ensure_resume_binding(
            &binding,
            ResumeRequest {
                user_id: 100,
                host_id: 200,
                connect_type: "SSH",
            },
        )
        .expect("must pass");
    }
}
