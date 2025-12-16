// 用途：QPS性能测量trait
// 说明：用于测量代码的性能指标，包括每秒查询数、总耗时和平均耗时
pub trait QPS {
    // 用途：计算并打印每秒查询数(QPS)
    // 说明：用于评估系统的吞吐量
    fn qps(&self, total: u64);
    // 用途：计算并打印总耗时和每个操作的平均耗时
    // 说明：用于评估系统的响应时间
    fn time(&self, total: u64);
    // 用途：打印总耗时
    // 说明：用于简单的性能测量
    fn cost(&self);
}

// 用途：为Instant类型实现QPS trait
// 说明：允许使用Instant类型直接测量和打印性能指标
impl QPS for std::time::Instant {
    // 用途：计算并打印每秒查询数(QPS)
    // 说明：通过总操作数和耗时计算QPS，评估系统吞吐量
    fn qps(&self, total: u64) {
        // 用途：获取耗时
        // 说明：计算QPS需要总耗时
        let time = self.elapsed();
        // 用途：打印QPS
        // 说明：将计算结果输出到控制台，方便查看
        println!(
            "use QPS: {} QPS/s",
            (total as u128 * 1000000000 as u128 / time.as_nanos() as u128)
        );
    }

    // 用途：计算并打印总耗时和每个操作的平均耗时
    // 说明：评估系统的响应时间，帮助优化性能
    fn time(&self, total: u64) {
        // 用途：获取耗时
        // 说明：计算平均耗时需要总耗时
        let time = self.elapsed();
        // 用途：打印总耗时和平均耗时
        // 说明：将计算结果输出到控制台，方便查看
        println!(
            "use Time: {:?} ,each:{}\n/op",
            &time,
            time.as_nanos() / (total as u128)
        );
    }

    // 用途：打印总耗时
    // 说明：简单测量代码执行时间
    fn cost(&self) {
        // 用途：获取耗时
        // 说明：打印总耗时需要获取耗时
        let time = self.elapsed();
        // 用途：打印总耗时
        // 说明：将耗时输出到控制台，方便查看
        println!("cost:{:?}", time);
    }
}
