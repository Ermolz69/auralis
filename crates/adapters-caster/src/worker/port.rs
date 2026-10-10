use super::*;

#[async_trait]
impl CasterPort for CasterWorkerAdapter {
    async fn catalog_page(
        &self,
        project: &ProjectId,
        after: Option<String>,
        limit: u32,
    ) -> Result<VoiceCatalogPage, PortError> {
        let payload = self.payload_with_store(json!({"after": after, "limit": limit}))?;
        self.call(project, "catalog_page", payload, Uuid::new_v4().to_string())
            .await
    }
    async fn inspect_voice(
        &self,
        project: &ProjectId,
        voice_id: String,
        after_reference: Option<String>,
        limit: u32,
    ) -> Result<VoiceDetails, PortError> {
        let payload = self.payload_with_store(
            json!({"voice_id": voice_id, "after_reference": after_reference, "limit": limit}),
        )?;
        self.call(
            project,
            "inspect_voice",
            payload,
            Uuid::new_v4().to_string(),
        )
        .await
    }
    async fn inspect_reference(
        &self,
        project: &ProjectId,
        voice_id: String,
        reference_id: String,
    ) -> Result<ReferenceDetails, PortError> {
        let payload =
            self.payload_with_store(json!({"voice_id": voice_id, "reference_id": reference_id}))?;
        self.call(
            project,
            "inspect_reference",
            payload,
            Uuid::new_v4().to_string(),
        )
        .await
    }
    async fn check_compatibility(
        &self,
        project: &ProjectId,
        voice_id: String,
        engine_id: String,
        emotion_id: String,
    ) -> Result<VoiceCompatibility, PortError> {
        let payload = self.payload_with_store(
            json!({"voice_id": voice_id, "engine_id": engine_id, "emotion_id": emotion_id}),
        )?;
        self.call(
            project,
            "check_compatibility",
            payload,
            Uuid::new_v4().to_string(),
        )
        .await
    }
    async fn current_cast(&self, project: &ProjectId) -> Result<Option<CastResult>, PortError> {
        if !self.existing_project_directory(project).await? {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct Current {
            current: Option<CastResult>,
        }
        let current: Current = self
            .call(
                project,
                "current_cast",
                json!({"project": self.project_path(project)}),
                Uuid::new_v4().to_string(),
            )
            .await?;
        Ok(current.current)
    }
    async fn assign_roles(
        &self,
        project: &ProjectId,
        cast: LogicalCast,
        selections: BTreeMap<String, ManualSelection>,
        expected_revision: Option<String>,
        request_id: String,
    ) -> Result<CastResult, PortError> {
        self.ensure_project_directory(project).await?;
        let payload = self.payload_with_store(json!({"project": self.project_path(project), "cast": cast, "selections": selections, "expected_revision": expected_revision}))?;
        self.call(project, "assign_roles", payload, request_id)
            .await
    }
    async fn recover_cast(&self, project: &ProjectId) -> Result<String, PortError> {
        if !self.existing_project_directory(project).await? {
            return Ok("not_needed".into());
        }
        #[derive(Deserialize)]
        struct Status {
            status: String,
        }
        let response: Status = self
            .call(
                project,
                "recover_cast",
                json!({"project": self.project_path(project)}),
                Uuid::new_v4().to_string(),
            )
            .await?;
        Ok(response.status)
    }
}
