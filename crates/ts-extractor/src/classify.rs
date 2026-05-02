use cgraph_core::SymbolKind;

/// Reclassify a Function as Hook if it follows React hook naming convention.
/// Hook: starts with "use" and 4th character is uppercase (per D-32).
/// Example: useCurrentUser -> Hook, useState -> Hook, useful -> Function
pub fn classify_function(name: &str) -> SymbolKind {
    if name.starts_with("use")
        && name.len() > 3
        && name.chars().nth(3).is_some_and(|c| c.is_uppercase())
    {
        SymbolKind::Hook
    } else {
        SymbolKind::Function
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_detection() {
        assert_eq!(classify_function("useCurrentUser"), SymbolKind::Hook);
        assert_eq!(classify_function("useState"), SymbolKind::Hook);
        assert_eq!(classify_function("useEffect"), SymbolKind::Hook);
        assert_eq!(classify_function("useToggle"), SymbolKind::Hook);
    }

    #[test]
    fn non_hook_functions() {
        assert_eq!(classify_function("fetchUser"), SymbolKind::Function);
        assert_eq!(classify_function("getUserById"), SymbolKind::Function);
        assert_eq!(classify_function("useful"), SymbolKind::Function); // 4th char 'f' not uppercase
        assert_eq!(classify_function("use"), SymbolKind::Function);    // too short
        assert_eq!(classify_function("profileCard"), SymbolKind::Function);
    }
}
