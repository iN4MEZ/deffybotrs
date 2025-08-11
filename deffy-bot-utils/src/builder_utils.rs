use serenity::all::{ActionRowComponent, CreateActionRow, CreateInputText, CreateInteractionResponse, CreateModal, InputTextStyle, ModalInteraction};

pub struct ModalBuilder {
    modal: CreateModal,
    components: Vec<CreateActionRow>,
}

impl ModalBuilder {
    pub fn new(custom_id: &str, title: &str) -> Self {
        Self {
            modal: CreateModal::new(custom_id, title),
            components: vec![],
        }
    }

    pub fn add_text_input(mut self, id: &str, label: &str, style: InputTextStyle) -> Self {
        let input = CreateInputText::new(style, label, id);
        let row = CreateActionRow::InputText(input);
        self.components.push(row);
        self
    }

    pub fn build(mut self) -> CreateInteractionResponse {
        self.modal = self.modal.components(self.components);
        CreateInteractionResponse::Modal(self.modal)
    }

    pub fn extract_modal_inputs(modal: &ModalInteraction) -> Vec<(String, String)> {
        modal
            .data
            .components
            .iter()
            .flat_map(|row| {
                row.components.iter().filter_map(|component| {
                    if let ActionRowComponent::InputText(input) = component {
                        Some((
                            input.custom_id.clone(),
                            input.value.clone().unwrap_or_default(),
                        ))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}