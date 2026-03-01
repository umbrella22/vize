use vize_relief::ErrorCode;

use super::{
    char_codes::{
        AT, COLON, DASH, DOT, DOUBLE_QUOTE, EQ, EXCLAMATION_MARK, GT, LEFT_SQUARE, LOWER_V, LT,
        NUMBER, QUESTION_MARK, RIGHT_SQUARE, SINGLE_QUOTE, SLASH,
    },
    types::{is_end_of_tag_section, is_tag_start_char, is_whitespace, Callbacks, QuoteType, State},
    Tokenizer,
};

impl<'a, C: Callbacks> Tokenizer<'a, C> {
    pub(super) fn cleanup(&mut self) {
        if self.section_start < self.index {
            match self.state {
                State::Text | State::Interpolation => {
                    self.callbacks.on_text(self.section_start, self.index);
                }
                State::InTagName
                | State::InSFCRootTagName
                | State::BeforeClosingTagName
                | State::InClosingTagName
                | State::BeforeAttrName
                | State::InAttrName
                | State::InDirName
                | State::InDirArg
                | State::InDirDynamicArg
                | State::InDirModifier
                | State::AfterAttrName
                | State::BeforeAttrValue
                | State::InAttrValueDq
                | State::InAttrValueSq
                | State::InAttrValueNq => {
                    self.callbacks.on_error(ErrorCode::EofInTag, self.index);
                }
                State::InCommentLike => {
                    self.callbacks.on_error(ErrorCode::EofInComment, self.index);
                    self.callbacks.on_comment(self.section_start, self.index);
                }
                _ => {}
            }
        }
    }

    // ========== State handlers ==========

    pub(super) fn state_text(&mut self, c: u8) {
        if c == LT {
            if self.index > self.section_start {
                self.callbacks.on_text(self.section_start, self.index);
            }
            self.state = State::BeforeTagName;
            self.section_start = self.index;
        } else if !self.callbacks.is_in_v_pre() && c == self.delimiter_open[0] {
            self.state = State::InterpolationOpen;
            self.delimiter_index = 0;
            self.state_interpolation_open(c);
        }
    }

    pub(super) fn state_interpolation_open(&mut self, c: u8) {
        if c == self.delimiter_open[self.delimiter_index] {
            self.delimiter_index += 1;
            if self.delimiter_index == self.delimiter_open.len() {
                let start = self.index + 1 - self.delimiter_open.len();
                if start > self.section_start {
                    self.callbacks.on_text(self.section_start, start);
                }
                self.section_start = self.index + 1;
                self.state = State::Interpolation;
                self.delimiter_index = 0;
            }
        } else {
            self.state = State::Text;
            self.state_text(c);
        }
    }

    pub(super) fn state_interpolation(&mut self, c: u8) {
        if c == self.delimiter_close[0] {
            self.state = State::InterpolationClose;
            self.delimiter_index = 0;
            self.state_interpolation_close(c);
        }
    }

    pub(super) fn state_interpolation_close(&mut self, c: u8) {
        if c == self.delimiter_close[self.delimiter_index] {
            self.delimiter_index += 1;
            if self.delimiter_index == self.delimiter_close.len() {
                self.callbacks.on_interpolation(
                    self.section_start,
                    self.index + 1 - self.delimiter_close.len(),
                );
                self.section_start = self.index + 1;
                self.state = State::Text;
            }
        } else {
            self.state = State::Interpolation;
            self.state_interpolation(c);
        }
    }

    pub(super) fn state_before_tag_name(&mut self, c: u8) {
        if c == EXCLAMATION_MARK {
            self.state = State::BeforeDeclaration;
            self.section_start = self.index + 1;
        } else if c == QUESTION_MARK {
            self.state = State::InProcessingInstruction;
            self.section_start = self.index + 1;
        } else if is_tag_start_char(c) {
            self.section_start = self.index;
            self.state = State::InTagName;
        } else if c == SLASH {
            self.state = State::BeforeClosingTagName;
        } else {
            self.state = State::Text;
            self.state_text(c);
        }
    }

    pub(super) fn state_in_tag_name(&mut self, c: u8) {
        if is_end_of_tag_section(c) {
            self.callbacks
                .on_open_tag_name(self.section_start, self.index);
            self.section_start = self.index;
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        }
    }

    pub(super) fn state_in_self_closing_tag(&mut self, c: u8) {
        if c == GT {
            self.callbacks.on_self_closing_tag(self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        } else if !is_whitespace(c) {
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        }
    }

    pub(super) fn state_before_closing_tag_name(&mut self, c: u8) {
        if is_whitespace(c) {
            // Skip
        } else if c == GT {
            self.callbacks
                .on_error(ErrorCode::MissingEndTagName, self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InClosingTagName;
            self.section_start = self.index;
        }
    }

    pub(super) fn state_in_closing_tag_name(&mut self, c: u8) {
        if c == GT || is_whitespace(c) {
            self.callbacks.on_close_tag(self.section_start, self.index);
            self.section_start = self.index + 1;
            self.state = if c == GT {
                State::Text
            } else {
                State::AfterClosingTagName
            };
        }
    }

    pub(super) fn state_after_closing_tag_name(&mut self, c: u8) {
        if c == GT {
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_before_attr_name(&mut self, c: u8) {
        if c == GT {
            self.callbacks.on_open_tag_end(self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        } else if c == SLASH {
            self.state = State::InSelfClosingTag;
        } else if !is_whitespace(c) {
            self.handle_attr_start(c);
        }
    }

    pub(super) fn handle_attr_start(&mut self, c: u8) {
        if self.callbacks.is_in_v_pre() {
            self.state = State::InAttrName;
            self.section_start = self.index;
            return;
        }
        if c == LOWER_V && self.index + 1 < self.input.len() && self.input[self.index + 1] == DASH {
            self.state = State::InDirName;
            self.section_start = self.index;
        } else if c == DOT || c == COLON || c == AT || c == NUMBER {
            self.callbacks.on_dir_name(self.index, self.index + 1);
            self.state = State::InDirArg;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InAttrName;
            self.section_start = self.index;
        }
    }

    pub(super) fn state_in_attr_name(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            self.callbacks
                .on_attrib_name(self.section_start, self.index);
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        }
    }

    pub(super) fn state_in_dir_name(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        } else if c == COLON {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.state = State::InDirArg;
            self.section_start = self.index + 1;
        } else if c == DOT {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.state = State::InDirModifier;
            self.section_start = self.index + 1;
        } else if c == LEFT_SQUARE {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.state = State::InDirDynamicArg;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_in_dir_arg(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            if self.section_start < self.index {
                self.callbacks.on_dir_arg(self.section_start, self.index);
            }
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        } else if c == LEFT_SQUARE {
            if self.section_start < self.index {
                self.callbacks.on_dir_arg(self.section_start, self.index);
            }
            self.state = State::InDirDynamicArg;
            self.section_start = self.index + 1;
        } else if c == DOT {
            if self.section_start < self.index {
                self.callbacks.on_dir_arg(self.section_start, self.index);
            }
            self.state = State::InDirModifier;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_in_dir_dynamic_arg(&mut self, c: u8) {
        if c == RIGHT_SQUARE {
            self.callbacks.on_dir_arg(self.section_start, self.index);
            self.state = State::InDirArg;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_in_dir_modifier(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            self.callbacks
                .on_dir_modifier(self.section_start, self.index);
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        } else if c == DOT {
            self.callbacks
                .on_dir_modifier(self.section_start, self.index);
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_after_attr_name(&mut self, c: u8) {
        if c == EQ {
            self.state = State::BeforeAttrValue;
        } else if c == SLASH || c == GT {
            self.callbacks.on_attrib_end(QuoteType::NoValue, self.index);
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        } else if !is_whitespace(c) {
            self.callbacks.on_attrib_end(QuoteType::NoValue, self.index);
            self.handle_attr_start(c);
        }
    }

    pub(super) fn state_before_attr_value(&mut self, c: u8) {
        if c == DOUBLE_QUOTE {
            self.state = State::InAttrValueDq;
            self.section_start = self.index + 1;
        } else if c == SINGLE_QUOTE {
            self.state = State::InAttrValueSq;
            self.section_start = self.index + 1;
        } else if !is_whitespace(c) {
            self.section_start = self.index;
            self.state = State::InAttrValueNq;
            self.state_in_attr_value_nq(c);
        }
    }

    pub(super) fn state_in_attr_value_dq(&mut self, c: u8) {
        if c == DOUBLE_QUOTE {
            self.emit_attr_value(QuoteType::Double);
        }
    }

    pub(super) fn state_in_attr_value_sq(&mut self, c: u8) {
        if c == SINGLE_QUOTE {
            self.emit_attr_value(QuoteType::Single);
        }
    }

    pub(super) fn state_in_attr_value_nq(&mut self, c: u8) {
        if is_whitespace(c) || c == GT {
            self.emit_attr_value(QuoteType::Unquoted);
            self.state_before_attr_name(c);
        } else if c == SLASH {
            self.emit_attr_value(QuoteType::Unquoted);
        }
    }

    pub(super) fn emit_attr_value(&mut self, quote: QuoteType) {
        if self.section_start < self.index {
            self.callbacks
                .on_attrib_data(self.section_start, self.index);
        }
        self.callbacks.on_attrib_end(quote, self.index);
        self.section_start = self.index + 1;
        self.state = State::BeforeAttrName;
    }

    pub(super) fn state_before_declaration(&mut self, c: u8) {
        if c == DASH {
            self.state = State::BeforeComment;
            self.section_start = self.index + 1;
        } else if c == LEFT_SQUARE {
            self.state = State::CDATASequence;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InDeclaration;
        }
    }

    pub(super) fn state_in_declaration(&mut self, c: u8) {
        if c == GT {
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_in_processing_instruction(&mut self, c: u8) {
        if c == GT {
            self.callbacks
                .on_processing_instruction(self.section_start, self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_before_comment(&mut self, c: u8) {
        if c == DASH {
            self.state = State::InCommentLike;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InDeclaration;
        }
    }

    pub(super) fn state_cdata_sequence(&mut self, _c: u8) {
        // TODO: Implement CDATA handling
        self.state = State::InCommentLike;
    }

    pub(super) fn state_in_special_comment(&mut self, c: u8) {
        if c == GT {
            self.callbacks.on_comment(self.section_start, self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_in_comment_like(&mut self, c: u8) {
        if c == DASH
            && self.index + 2 < self.input.len()
            && self.input[self.index + 1] == DASH
            && self.input[self.index + 2] == GT
        {
            self.callbacks.on_comment(self.section_start, self.index);
            self.index += 2;
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    pub(super) fn state_before_special_s(&mut self, _c: u8) {
        // TODO: Handle script/style
        self.state = State::InTagName;
    }

    pub(super) fn state_before_special_t(&mut self, _c: u8) {
        // TODO: Handle title/textarea
        self.state = State::InTagName;
    }

    pub(super) fn state_special_start_sequence(&mut self, _c: u8) {
        self.state = State::InTagName;
    }

    pub(super) fn state_in_rcdata(&mut self, c: u8) {
        if c == LT {
            self.state = State::BeforeTagName;
        }
    }

    pub(super) fn state_in_entity(&mut self, _c: u8) {
        // TODO: Implement entity decoding
        self.state = State::Text;
    }

    pub(super) fn state_in_sfc_root_tag_name(&mut self, c: u8) {
        if is_end_of_tag_section(c) {
            self.callbacks
                .on_open_tag_name(self.section_start, self.index);
            self.section_start = self.index;
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        }
    }
}
