pub fn choose(
    models: &Models,
    model: Option<ModelId>,
    effort: Option<Effort>,
) -> Result<Choice, String> {
    let model = match model {
        Some(model) => models
            .find(&model)
            .map(|choice| Some(choice.id.clone()))
            .ok_or_else(|| format!("this host doesn't offer the model {}", model.0)),
        None => Ok(models.default.clone()),
    }?;
    let listed = model.as_ref().and_then(|model| models.find(model));
    match (effort, listed) {
        (Some(effort), Some(choice)) => choice
            .efforts
            .iter()
            .find(|offered| offered == &&effort)
            .map(|_| Choice {
                model: model.clone(),
                effort: Some(effort),
            })
            .ok_or_else(|| format!("{} doesn't take the effort {}", choice.name, effort.wire())),
        (effort, _) => Ok(Choice { model, effort }),
    }
}
