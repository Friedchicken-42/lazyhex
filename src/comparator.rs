use ratatui::style::Color;

use crate::viewer::{Highlight, Viewer};

pub struct Comparator<'a> {
    pub viewer_old: Viewer<'a>,
    pub viewer_new: Viewer<'a>,
    pub added: usize,
    pub deleted: usize,
    pub replaced: usize,
}

impl<'a> Comparator<'a> {
    pub fn new(
        data_old: &'a mut Vec<Option<u8>>,
        data_new: &'a mut Vec<Option<u8>>,
        file_old: &'a str,
        file_new: &'a str,
    ) -> Self {
        let diffs = similar::capture_diff_slices(similar::Algorithm::Myers, data_old, data_new);

        let mut viewer_old = Viewer::new(data_old, Some(file_old));
        let mut viewer_new = Viewer::new(data_new, Some(file_new));

        let mut added = 0;
        let mut deleted = 0;
        let mut replaced = 0;

        let mut offset_old = 0;
        let mut offset_new = 0;

        for diff in diffs {
            match diff {
                similar::DiffOp::Equal { .. } => {}
                similar::DiffOp::Delete {
                    old_index,
                    old_len,
                    new_index,
                } => {
                    let highlight = Highlight {
                        start: old_index + offset_old,
                        end: old_index + old_len + offset_old - 1,
                        bg: Color::Red,
                        fg: Color::White,
                    };
                    for _ in 0..old_len {
                        viewer_new.data.insert(new_index + offset_new, None);
                    }
                    offset_new += old_len;
                    viewer_old.highlights.push(highlight);
                    deleted += old_len;
                }
                similar::DiffOp::Insert {
                    new_index,
                    new_len,
                    old_index,
                } => {
                    let highlight = Highlight {
                        start: new_index + offset_new,
                        end: new_index + new_len + offset_new - 1,
                        bg: Color::Green,
                        fg: Color::White,
                    };
                    for _ in 0..new_len {
                        viewer_old.data.insert(old_index + offset_old, None);
                    }
                    offset_old += new_len;
                    viewer_new.highlights.push(highlight);
                    added += new_len;
                }
                similar::DiffOp::Replace {
                    old_index,
                    old_len,
                    new_index,
                    new_len,
                } => {
                    let highlight = Highlight {
                        start: old_index + offset_old,
                        end: old_index + old_len + offset_old - 1,
                        bg: Color::Yellow,
                        fg: Color::Black,
                    };
                    viewer_old.highlights.push(highlight);

                    let highlight = Highlight {
                        start: new_index + offset_new,
                        end: new_index + new_len + offset_new - 1,
                        bg: Color::Yellow,
                        fg: Color::Black,
                    };
                    viewer_new.highlights.push(highlight);

                    if new_len > old_len {
                        let distance = new_len - old_len;
                        for _ in 0..distance {
                            viewer_old
                                .data
                                .insert(old_index + old_len + offset_old, None);
                        }
                        offset_old += distance;
                        replaced += distance;
                    } else {
                        let distance = old_len - new_len;
                        for _ in 0..distance {
                            viewer_new
                                .data
                                .insert(new_index + new_len + offset_new, None);
                        }
                        offset_new += distance;
                        replaced += distance;
                    }
                }
            }
        }

        Self {
            viewer_old,
            viewer_new,
            added,
            deleted,
            replaced,
        }
    }

    pub fn left(&mut self) {
        self.viewer_old.left();
        self.viewer_new.left();
    }

    pub fn right(&mut self) {
        self.viewer_old.right();
        self.viewer_new.right();
    }

    pub fn up(&mut self) {
        self.viewer_old.up();
        self.viewer_new.up();
    }

    pub fn down(&mut self) {
        self.viewer_old.down();
        self.viewer_new.down();
    }
}
