use brul_gui::ClipboardData;

pub trait Clipboard {
    fn read(&self) -> Option<ClipboardData>;
    fn write(&self, data: &ClipboardData);
}
