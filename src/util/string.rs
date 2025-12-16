// 用途：字符串为空检查trait
// 说明：用于检查字符串或可选字符串是否为空，统一空值检查逻辑
pub trait IsEmptyString {
    // 用途：检查字符串是否为空
    // 说明：提供统一的空值检查方法，简化代码
    fn is_empty(&self) -> bool;
}

// 用途：为Option<String>实现IsEmptyString trait
// 说明：方便检查Option<String>是否为空，包括None和空字符串情况
impl IsEmptyString for Option<String> {
    // 用途：检查Option<String>是否为空
    // 说明：None或空字符串都返回true，便于统一处理空值情况
    fn is_empty(&self) -> bool {
        match self {
            // 用途：如果有值，检查字符串是否为空
            // 说明：空字符串也视为空值
            Some(s) => s.is_empty(),
            // 用途：如果没有值，视为空
            // 说明：None表示缺少值，应视为空
            _ => true,
        }
    }
}

// 用途：为Option<&str>实现IsEmptyString trait
// 说明：方便检查Option<&str>是否为空，包括None和空字符串情况
impl IsEmptyString for Option<&str> {
    // 用途：检查Option<&str>是否为空
    // 说明：None或空字符串都返回true，便于统一处理空值情况
    fn is_empty(&self) -> bool {
        match self {
            // 用途：如果有值，检查字符串是否为空
            // 说明：空字符串也视为空值
            Some(s) => s.is_empty(),
            // 用途：如果没有值，视为空
            // 说明：None表示缺少值，应视为空
            _ => true,
        }
    }
}
