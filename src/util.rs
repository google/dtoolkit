use core::fmt::{self, Write};

pub(crate) fn write_indent(f: &mut impl Write, indent: usize) -> fmt::Result {
    for _ in 0..indent {
        f.write_char(' ')?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_indent;

    #[test]
    fn large_indentation_does_not_panic() {
        let mut output = String::new();
        write_indent(&mut output, usize::from(u16::MAX) + 1).unwrap();
        assert_eq!(output.len(), usize::from(u16::MAX) + 1);
    }
}

#[must_use]
pub(crate) fn name_without_address(name: &str) -> &str {
    if let Some((name, _)) = name.split_once('@') {
        name
    } else {
        name
    }
}
