#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_tool_execution() {
        // Create a tool registry with built-in tools
        let mut registry = ToolRegistry::new().with_builtin_tools();

        // Test echo tool
        let echo_result = registry.execute("echo", "Hello, World!").unwrap();
        assert_eq!(echo_result, "Hello, World!");

        // Test add tool
        let add_result = registry.execute("add", "1, 2, 3, 4, 5").unwrap();
        assert_eq!(add_result, "15");

        // Test unknown tool
        let unknown_result = registry.execute("unknown", "test");
        assert!(unknown_result.is_err());
    }

    #[tokio::test]
    async fn test_tool_permissions() {
        // Create a tool registry with strict permissions
        let registry = ToolRegistry::new()
            .with_permission_policy(PermissionPolicy::new(PermissionMode::None));

        // Check if tool requires permission
        let requires_perm = registry.requires_permission("echo");
        assert!(requires_perm);

        // Test authorization
        let auth_result = registry.authorize("echo", "test");
        assert!(auth_result.is_err());
    }

    #[tokio::test]
    async fn test_tool_registration() {
        // Create a tool registry
        let mut registry = ToolRegistry::new();

        // Register a custom tool
        registry = registry.register("multiply", |input| {
            let numbers: Result<Vec<i32>, _> = input
                .split(',')
                .map(|s| s.trim().parse())
                .collect();
            match numbers {
                Ok(nums) if !nums.is_empty() => {
                    let product = nums.iter().product::<i32>();
                    Ok(product.to_string())
                },
                _ => Err(ToolError::new("Invalid input")),
            }
        });

        // Test the custom tool
        let multiply_result = registry.execute("multiply", "2, 3, 4").unwrap();
        assert_eq!(multiply_result, "24");
    }

    #[tokio::test]
    async fn test_list_tools() {
        // Create a tool registry with built-in tools
        let registry = ToolRegistry::new().with_builtin_tools();

        // List tools
        let tools = registry.list_tools();
        assert!(tools.contains(&"echo".to_string()));
        assert!(tools.contains(&"add".to_string()));
        assert_eq!(tools.len(), 2);
    }
}