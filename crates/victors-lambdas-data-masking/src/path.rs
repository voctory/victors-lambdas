//! Field path normalization and resolution.

use crate::{DataMaskingError, DataMaskingResult};
use serde_json::Value;

pub(crate) fn matching_json_pointers(data: &Value, path: &str) -> DataMaskingResult<Vec<String>> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(DataMaskingError::invalid_path(path));
    }

    if trimmed.starts_with('/') || !requires_json_path_resolution(trimmed) {
        return Ok(vec![to_json_pointer(trimmed)?]);
    }

    let selectors = parse_json_path(trimmed).map_err(|()| DataMaskingError::invalid_path(path))?;
    let mut pointers = resolve_json_path(data, &selectors);
    pointers.sort_unstable_by(|left, right| {
        pointer_depth(right)
            .cmp(&pointer_depth(left))
            .then_with(|| left.cmp(right))
    });
    pointers.dedup();

    Ok(pointers)
}

pub(crate) fn to_json_pointer(path: &str) -> DataMaskingResult<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(DataMaskingError::invalid_path(path));
    }

    if trimmed.starts_with('/') {
        return Ok(trimmed.to_string());
    }

    let mut pointer = String::new();
    for segment in trimmed.split('.') {
        if segment.is_empty() {
            return Err(DataMaskingError::invalid_path(path));
        }

        pointer.push('/');
        pointer.push_str(&escape_pointer_segment(segment));
    }

    Ok(pointer)
}

fn requires_json_path_resolution(path: &str) -> bool {
    path == "$"
        || path.starts_with("$.")
        || path.starts_with("$[")
        || path.starts_with("$..")
        || path.contains("..")
        || path.contains('[')
        || path.contains(".'")
        || path.contains(".\"")
        || path.contains('*')
}

fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn append_pointer_segment(pointer: &str, segment: &str) -> String {
    let escaped = escape_pointer_segment(segment);
    if pointer.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{pointer}/{escaped}")
    }
}

fn pointer_depth(pointer: &str) -> usize {
    pointer.bytes().filter(|byte| *byte == b'/').count()
}

#[derive(Debug, PartialEq)]
enum Selector {
    Child(String),
    Index(usize),
    Wildcard,
    Recursive(String),
    RecursiveWildcard,
    Filter(FilterPredicate),
}

#[derive(Debug, PartialEq)]
struct FilterPredicate {
    path: Vec<FilterSegment>,
    comparison: Option<FilterComparison>,
}

impl FilterPredicate {
    fn matches(&self, value: &Value) -> bool {
        let Some(candidate) = select_filter_value(value, &self.path) else {
            return false;
        };

        self.comparison.as_ref().map_or_else(
            || is_truthy(candidate),
            |comparison| comparison.matches(candidate),
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
enum FilterSegment {
    Child(String),
    Index(usize),
}

#[derive(Debug, PartialEq)]
struct FilterComparison {
    op: ComparisonOp,
    value: FilterValue,
}

impl FilterComparison {
    fn matches(&self, actual: &Value) -> bool {
        match self.op {
            ComparisonOp::Eq => self.value.equals(actual),
            ComparisonOp::NotEq => !self.value.equals(actual),
            ComparisonOp::GreaterThan => self
                .value
                .ordering(actual)
                .is_some_and(std::cmp::Ordering::is_gt),
            ComparisonOp::GreaterOrEqual => self
                .value
                .ordering(actual)
                .is_some_and(std::cmp::Ordering::is_ge),
            ComparisonOp::LessThan => self
                .value
                .ordering(actual)
                .is_some_and(std::cmp::Ordering::is_lt),
            ComparisonOp::LessOrEqual => self
                .value
                .ordering(actual)
                .is_some_and(std::cmp::Ordering::is_le),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComparisonOp {
    Eq,
    NotEq,
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
}

#[derive(Debug, PartialEq)]
enum FilterValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl FilterValue {
    fn equals(&self, actual: &Value) -> bool {
        match (self, actual) {
            (Self::String(expected), Value::String(actual)) => actual == expected,
            (Self::Number(expected), Value::Number(actual)) => actual
                .as_f64()
                .is_some_and(|actual| (actual - expected).abs() <= f64::EPSILON),
            (Self::Bool(expected), Value::Bool(actual)) => actual == expected,
            (Self::Null, Value::Null) => true,
            _ => false,
        }
    }

    fn ordering(&self, actual: &Value) -> Option<std::cmp::Ordering> {
        match self {
            Self::Number(expected) => actual.as_f64()?.partial_cmp(expected),
            Self::String(expected) => actual.as_str()?.partial_cmp(expected.as_str()),
            Self::Bool(_) | Self::Null => None,
        }
    }
}

#[derive(Debug)]
struct JsonPathMatch<'a> {
    pointer: String,
    value: &'a Value,
}

fn parse_json_path(path: &str) -> Result<Vec<Selector>, ()> {
    JsonPathParser::new(path).parse()
}

fn resolve_json_path(data: &Value, selectors: &[Selector]) -> Vec<String> {
    let mut matches = vec![JsonPathMatch {
        pointer: String::new(),
        value: data,
    }];

    for selector in selectors {
        let mut next_matches = Vec::new();
        for matched in &matches {
            apply_selector(matched, selector, &mut next_matches);
        }
        matches = next_matches;

        if matches.is_empty() {
            break;
        }
    }

    matches.into_iter().map(|matched| matched.pointer).collect()
}

fn apply_selector<'a>(
    matched: &JsonPathMatch<'a>,
    selector: &Selector,
    output: &mut Vec<JsonPathMatch<'a>>,
) {
    match selector {
        Selector::Child(name) => push_child(&matched.pointer, matched.value, name, output),
        Selector::Index(index) => push_index(&matched.pointer, matched.value, *index, output),
        Selector::Wildcard => push_wildcard(&matched.pointer, matched.value, output),
        Selector::Recursive(name) => {
            collect_recursive_child(&matched.pointer, matched.value, name, output);
        }
        Selector::RecursiveWildcard => {
            collect_recursive_wildcard(&matched.pointer, matched.value, output);
        }
        Selector::Filter(predicate) => {
            push_filtered(&matched.pointer, matched.value, predicate, output);
        }
    }
}

fn push_child<'a>(
    pointer: &str,
    value: &'a Value,
    name: &str,
    output: &mut Vec<JsonPathMatch<'a>>,
) {
    match value {
        Value::Object(fields) => {
            if let Some(child) = fields.get(name) {
                push_match(pointer, name, child, output);
            }
        }
        Value::Array(items) => {
            if let Ok(index) = name.parse::<usize>() {
                if let Some(child) = items.get(index) {
                    let segment = index.to_string();
                    push_match(pointer, &segment, child, output);
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn push_index<'a>(
    pointer: &str,
    value: &'a Value,
    index: usize,
    output: &mut Vec<JsonPathMatch<'a>>,
) {
    if let Value::Array(items) = value {
        if let Some(child) = items.get(index) {
            let segment = index.to_string();
            push_match(pointer, &segment, child, output);
        }
    }
}

fn push_wildcard<'a>(pointer: &str, value: &'a Value, output: &mut Vec<JsonPathMatch<'a>>) {
    match value {
        Value::Object(fields) => {
            for (name, child) in fields {
                push_match(pointer, name, child, output);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let segment = index.to_string();
                push_match(pointer, &segment, child, output);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn push_filtered<'a>(
    pointer: &str,
    value: &'a Value,
    predicate: &FilterPredicate,
    output: &mut Vec<JsonPathMatch<'a>>,
) {
    match value {
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                if predicate.matches(child) {
                    let segment = index.to_string();
                    push_match(pointer, &segment, child, output);
                }
            }
        }
        Value::Object(fields) => {
            for (name, child) in fields {
                if predicate.matches(child) {
                    push_match(pointer, name, child, output);
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn collect_recursive_child<'a>(
    pointer: &str,
    value: &'a Value,
    name: &str,
    output: &mut Vec<JsonPathMatch<'a>>,
) {
    match value {
        Value::Object(fields) => {
            if let Some(child) = fields.get(name) {
                push_match(pointer, name, child, output);
            }

            for (child_name, child) in fields {
                let child_pointer = append_pointer_segment(pointer, child_name);
                collect_recursive_child(&child_pointer, child, name, output);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let child_pointer = append_pointer_segment(pointer, &index.to_string());
                collect_recursive_child(&child_pointer, child, name, output);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn collect_recursive_wildcard<'a>(
    pointer: &str,
    value: &'a Value,
    output: &mut Vec<JsonPathMatch<'a>>,
) {
    match value {
        Value::Object(fields) => {
            for (name, child) in fields {
                let child_pointer = append_pointer_segment(pointer, name);
                output.push(JsonPathMatch {
                    pointer: child_pointer.clone(),
                    value: child,
                });
                collect_recursive_wildcard(&child_pointer, child, output);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let child_pointer = append_pointer_segment(pointer, &index.to_string());
                output.push(JsonPathMatch {
                    pointer: child_pointer.clone(),
                    value: child,
                });
                collect_recursive_wildcard(&child_pointer, child, output);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn push_match<'a>(
    pointer: &str,
    segment: &str,
    value: &'a Value,
    output: &mut Vec<JsonPathMatch<'a>>,
) {
    output.push(JsonPathMatch {
        pointer: append_pointer_segment(pointer, segment),
        value,
    });
}

fn select_filter_value<'a>(value: &'a Value, path: &[FilterSegment]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = match segment {
            FilterSegment::Child(name) => select_child(current, name)?,
            FilterSegment::Index(index) => current.as_array()?.get(*index)?,
        };
    }
    Some(current)
}

fn select_child<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => fields.get(name),
        Value::Array(items) => name
            .parse::<usize>()
            .ok()
            .and_then(|index| items.get(index)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

struct JsonPathParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> JsonPathParser<'a> {
    const fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn parse(&mut self) -> Result<Vec<Selector>, ()> {
        let root_explicit = self.consume("$");
        let mut selectors = Vec::new();

        while !self.is_end() {
            if self.consume("..") {
                selectors.push(self.parse_recursive_selector()?);
            } else if self.consume(".") {
                selectors.push(self.parse_dot_selector()?);
            } else if self.starts_with("[") {
                selectors.push(self.parse_bracket_selector()?);
            } else if !root_explicit && selectors.is_empty() {
                selectors.push(Selector::Child(self.parse_bare_member()?));
            } else {
                return Err(());
            }
        }

        Ok(selectors)
    }

    fn parse_recursive_selector(&mut self) -> Result<Selector, ()> {
        if self.consume("*") {
            return Ok(Selector::RecursiveWildcard);
        }

        let name = if self.next_is_quote() {
            self.parse_quoted_string()?
        } else {
            self.parse_bare_member()?
        };

        Ok(Selector::Recursive(name))
    }

    fn parse_dot_selector(&mut self) -> Result<Selector, ()> {
        if self.consume("*") {
            return Ok(Selector::Wildcard);
        }

        let name = if self.next_is_quote() {
            self.parse_quoted_string()?
        } else {
            self.parse_bare_member()?
        };

        Ok(Selector::Child(name))
    }

    fn parse_bracket_selector(&mut self) -> Result<Selector, ()> {
        self.expect("[")?;

        if self.consume("*") {
            self.expect("]")?;
            return Ok(Selector::Wildcard);
        }

        if self.consume("?(") {
            let condition = self.parse_filter_condition()?;
            self.expect("]")?;
            return parse_filter_predicate(&condition).map(Selector::Filter);
        }

        if self.next_is_quote() {
            let name = self.parse_quoted_string()?;
            self.expect("]")?;
            return Ok(Selector::Child(name));
        }

        let index = self.parse_array_index()?;
        self.expect("]")?;
        Ok(Selector::Index(index))
    }

    fn parse_filter_condition(&mut self) -> Result<String, ()> {
        let start = self.position;
        let mut quote = None;
        let mut escaped = false;

        while let Some(character) = self.advance_char() {
            if escaped {
                escaped = false;
                continue;
            }

            if character == '\\' {
                escaped = true;
                continue;
            }

            if let Some(quote_char) = quote {
                if character == quote_char {
                    quote = None;
                }
                continue;
            }

            if character == '\'' || character == '"' {
                quote = Some(character);
                continue;
            }

            if character == ')' {
                let end = self.position - character.len_utf8();
                return Ok(self.input[start..end].to_string());
            }
        }

        Err(())
    }

    fn parse_array_index(&mut self) -> Result<usize, ()> {
        let start = self.position;
        while self
            .peek_char()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance_char();
        }

        if self.position == start {
            return Err(());
        }

        self.input[start..self.position]
            .parse::<usize>()
            .map_err(|_| ())
    }

    fn parse_bare_member(&mut self) -> Result<String, ()> {
        let start = self.position;
        while let Some(character) = self.peek_char() {
            if character == '.' || character == '[' {
                break;
            }

            if character.is_whitespace()
                || character == ']'
                || character == ')'
                || character == '\''
                || character == '"'
            {
                return Err(());
            }

            self.advance_char();
        }

        if self.position == start {
            return Err(());
        }

        Ok(self.input[start..self.position].to_string())
    }

    fn parse_quoted_string(&mut self) -> Result<String, ()> {
        let Some(quote) = self.advance_char() else {
            return Err(());
        };

        if quote != '\'' && quote != '"' {
            return Err(());
        }

        let mut value = String::new();
        let mut escaped = false;

        while let Some(character) = self.advance_char() {
            if escaped {
                value.push(match character {
                    'b' => '\u{0008}',
                    'f' => '\u{000c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
                escaped = false;
                continue;
            }

            if character == '\\' {
                escaped = true;
                continue;
            }

            if character == quote {
                return Ok(value);
            }

            value.push(character);
        }

        Err(())
    }

    fn expect(&mut self, expected: &str) -> Result<(), ()> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn next_is_quote(&self) -> bool {
        self.peek_char()
            .is_some_and(|character| character == '\'' || character == '"')
    }

    fn consume(&mut self, expected: &str) -> bool {
        if self.starts_with(expected) {
            self.position += expected.len();
            true
        } else {
            false
        }
    }

    fn starts_with(&self, expected: &str) -> bool {
        self.remaining().starts_with(expected)
    }

    fn is_end(&self) -> bool {
        self.position == self.input.len()
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.position..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let character = self.peek_char()?;
        self.position += character.len_utf8();
        Some(character)
    }
}

fn parse_filter_predicate(condition: &str) -> Result<FilterPredicate, ()> {
    let condition = condition.trim();
    if condition.is_empty() {
        return Err(());
    }

    let Some((left, op, right)) = split_filter_comparison(condition)? else {
        return Ok(FilterPredicate {
            path: parse_filter_path(condition)?,
            comparison: None,
        });
    };

    Ok(FilterPredicate {
        path: parse_filter_path(left)?,
        comparison: Some(FilterComparison {
            op,
            value: parse_filter_value(right)?,
        }),
    })
}

fn split_filter_comparison(condition: &str) -> Result<Option<(&str, ComparisonOp, &str)>, ()> {
    let mut quote = None;
    let mut escaped = false;

    for (index, character) in condition.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if character == '\\' {
            escaped = true;
            continue;
        }

        if let Some(quote_char) = quote {
            if character == quote_char {
                quote = None;
            }
            continue;
        }

        if character == '\'' || character == '"' {
            quote = Some(character);
            continue;
        }

        let remaining = &condition[index..];
        for (symbol, op) in [
            ("==", ComparisonOp::Eq),
            ("!=", ComparisonOp::NotEq),
            (">=", ComparisonOp::GreaterOrEqual),
            ("<=", ComparisonOp::LessOrEqual),
            (">", ComparisonOp::GreaterThan),
            ("<", ComparisonOp::LessThan),
        ] {
            if remaining.starts_with(symbol) {
                let left = condition[..index].trim();
                let right = condition[index + symbol.len()..].trim();
                return if left.is_empty() || right.is_empty() {
                    Err(())
                } else {
                    Ok(Some((left, op, right)))
                };
            }
        }
    }

    if quote.is_some() || escaped {
        return Err(());
    }

    Ok(None)
}

fn parse_filter_path(path: &str) -> Result<Vec<FilterSegment>, ()> {
    let mut parser = JsonPathParser::new(path.trim());
    if !parser.consume("@") {
        return Err(());
    }

    let mut segments = Vec::new();
    while !parser.is_end() {
        if parser.consume(".") {
            let name = if parser.next_is_quote() {
                parser.parse_quoted_string()?
            } else {
                parser.parse_bare_member()?
            };
            segments.push(FilterSegment::Child(name));
        } else if parser.consume("[") {
            if parser.next_is_quote() {
                let name = parser.parse_quoted_string()?;
                parser.expect("]")?;
                segments.push(FilterSegment::Child(name));
            } else {
                let index = parser.parse_array_index()?;
                parser.expect("]")?;
                segments.push(FilterSegment::Index(index));
            }
        } else {
            return Err(());
        }
    }

    Ok(segments)
}

fn parse_filter_value(value: &str) -> Result<FilterValue, ()> {
    let value = value.trim();
    if value.is_empty() {
        return Err(());
    }

    let mut parser = JsonPathParser::new(value);
    if parser.next_is_quote() {
        let parsed = parser.parse_quoted_string()?;
        if parser.is_end() {
            return Ok(FilterValue::String(parsed));
        }
        return Err(());
    }

    match value {
        "true" => Ok(FilterValue::Bool(true)),
        "false" => Ok(FilterValue::Bool(false)),
        "null" => Ok(FilterValue::Null),
        _ => value
            .parse::<f64>()
            .map(FilterValue::Number)
            .map_err(|_| ()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_pointers(actual: &[String], expected: &[&str]) {
        let expected = expected
            .iter()
            .map(|pointer| (*pointer).to_string())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn keeps_json_pointer_paths() {
        let pointer = to_json_pointer("/customer/password").expect("path should parse");

        assert_eq!(pointer, "/customer/password");
    }

    #[test]
    fn converts_dot_paths() {
        let pointer = to_json_pointer("customer.password").expect("path should parse");

        assert_eq!(pointer, "/customer/password");
    }

    #[test]
    fn escapes_dot_path_segments() {
        let pointer = to_json_pointer("customer.api/key~1").expect("path should parse");

        assert_eq!(pointer, "/customer/api~1key~01");
    }

    #[test]
    fn rejects_empty_paths() {
        let error = to_json_pointer(" ").expect_err("path should fail");

        assert_eq!(error.kind(), crate::DataMaskingErrorKind::InvalidPath);
    }

    #[test]
    fn resolves_plain_dot_paths_without_traversal() {
        let pointers =
            matching_json_pointers(&json!({}), "items.0.card").expect("path should parse");

        assert_pointers(&pointers, &["/items/0/card"]);
    }

    #[test]
    fn keeps_json_pointer_paths_literal() {
        let pointers = matching_json_pointers(&json!({}), "/items[*]/card")
            .expect("pointer should stay literal");

        assert_pointers(&pointers, &["/items[*]/card"]);
    }

    #[test]
    fn resolves_root_jsonpath() {
        let pointers =
            matching_json_pointers(&json!({"name": "Ada"}), "$").expect("path should parse");

        assert_pointers(&pointers, &[""]);
    }

    #[test]
    fn resolves_wildcard_jsonpath() {
        let data = json!({
            "items": [
                {"card": "1111"},
                {"card": "2222"}
            ]
        });

        let pointers = matching_json_pointers(&data, "$.items[*].card").expect("path should parse");

        assert_pointers(&pointers, &["/items/0/card", "/items/1/card"]);
    }

    #[test]
    fn resolves_recursive_descent_jsonpath() {
        let data = json!({
            "records": [
                {"value": "one"},
                {"nested": {"value": "two"}}
            ],
            "value": "root"
        });

        let pointers = matching_json_pointers(&data, "$..value").expect("path should parse");

        assert_pointers(
            &pointers,
            &["/records/1/nested/value", "/records/0/value", "/value"],
        );
    }

    #[test]
    fn resolves_quoted_segments() {
        let data = json!({
            "a": {
                "1": {
                    "None": "value"
                },
                "nested": {
                    "4": "recursive"
                }
            }
        });

        let direct = matching_json_pointers(&data, "a.'1'.None").expect("path should parse");
        let recursive = matching_json_pointers(&data, "a..'4'").expect("path should parse");

        assert_pointers(&direct, &["/a/1/None"]);
        assert_pointers(&recursive, &["/a/nested/4"]);
    }

    #[test]
    fn resolves_filtered_jsonpath() {
        let data = json!({
            "other_address": [
                {"postcode": 90210, "line": "masked"},
                {"postcode": 1000, "line": "public"}
            ]
        });

        let pointers = matching_json_pointers(&data, "$.other_address[?(@.postcode > 12000)].line")
            .expect("path should parse");

        assert_pointers(&pointers, &["/other_address/0/line"]);
    }

    #[test]
    fn rejects_invalid_jsonpath() {
        let error = matching_json_pointers(&json!({}), "$.items[").expect_err("path should fail");

        assert_eq!(error.kind(), crate::DataMaskingErrorKind::InvalidPath);
    }
}
