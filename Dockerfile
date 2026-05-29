FROM rust:latest as builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /usr/src/rsllm

# 复制项目文件
COPY . .

# 设置环境变量解决 aws-lc-sys 问题
ENV AWS_LC_SYS_NO_PREFIX=1
ENV AWS_LC_SYS_NO_ASM=1
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# 构建项目
RUN cargo build --release

# 运行时镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 从构建阶段复制二进制文件
COPY --from=builder /usr/src/rsllm/target/release/rsllm /usr/local/bin/rsllm

# 设置执行权限
RUN chmod +x /usr/local/bin/rsllm

# 设置入口点
ENTRYPOINT ["/usr/local/bin/rsllm"]
