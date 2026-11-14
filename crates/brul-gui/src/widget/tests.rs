use super::WidgetError;
use crate::layout::LayoutError;
use crate::text::TextError;

#[test]
fn widget_error_names_the_wrapped_source() {
    assert_ne!(
        WidgetError::NoLayout.to_string(),
        WidgetError::InvalidScale.to_string()
    );
    assert_eq!(
        WidgetError::from(LayoutError::InvalidDimension).to_string(),
        format!("layout: {}", LayoutError::InvalidDimension)
    );
    assert_eq!(
        WidgetError::from(TextError::AtlasFull).to_string(),
        format!("text: {}", TextError::AtlasFull)
    );
}
