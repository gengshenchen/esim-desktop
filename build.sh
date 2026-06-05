#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

usage() {
    echo "E02T 产测工具编译脚本"
    echo ""
    echo "用法: ./build.sh <command> [options]"
    echo ""
    echo "命令:"
    echo "  dev             启动开发模式 (热重载)"
    echo "  build           本地编译 release 版本"
    echo "  clean           清理所有编译产物"
    echo "  rebuild         clean + build"
    echo "  check           类型检查 (Rust + TypeScript)"
    echo "  release <ver>   打 tag 并推送到 GitHub 触发 CI 编译跨平台版本"
    echo "                  示例: ./build.sh release 1.0.1"
    echo "  release-list    查看已有的 release tag"
    echo ""
    echo "示例:"
    echo "  ./build.sh dev              # 开发调试"
    echo "  ./build.sh build            # 本地编译 Linux 版本"
    echo "  ./build.sh rebuild          # 全量重新编译"
    echo "  ./build.sh release 1.0.1    # 推送 v1.0.1 到 GitHub, CI 自动编译 Linux + Windows"
    exit 0
}

ensure_deps() {
    if ! command -v pnpm &> /dev/null; then
        echo -e "${RED}pnpm 未安装，请先: npm install -g pnpm${NC}"
        exit 1
    fi
    if [ ! -d "node_modules" ]; then
        echo -e "${YELLOW}安装前端依赖...${NC}"
        pnpm install
    fi
}

cmd_dev() {
    ensure_deps
    echo -e "${GREEN}启动开发模式...${NC}"
    pnpm tauri dev
}

cmd_build() {
    ensure_deps
    echo -e "${GREEN}编译 release 版本...${NC}"
    pnpm tauri build
    echo ""
    echo -e "${GREEN}编译完成！产物:${NC}"
    ls -lh src-tauri/target/release/bundle/appimage/*.AppImage 2>/dev/null
    ls -lh src-tauri/target/release/bundle/deb/*.deb 2>/dev/null
    ls -lh src-tauri/target/release/bundle/rpm/*.rpm 2>/dev/null
}

cmd_clean() {
    echo -e "${YELLOW}清理编译产物...${NC}"
    rm -rf src-tauri/target
    rm -rf dist
    rm -rf node_modules
    echo -e "${GREEN}清理完成${NC}"
}

cmd_rebuild() {
    cmd_clean
    ensure_deps
    cmd_build
}

cmd_check() {
    ensure_deps
    echo -e "${YELLOW}TypeScript 检查...${NC}"
    npx vue-tsc --noEmit
    echo -e "${GREEN}TypeScript OK${NC}"

    echo -e "${YELLOW}Rust 检查...${NC}"
    cd src-tauri && cargo check 2>&1 | grep -E "^(error|warning:.*esim|.*Finished)" && cd ..
    echo -e "${GREEN}Rust OK${NC}"
}

cmd_release() {
    local version="$1"
    if [ -z "$version" ]; then
        echo -e "${RED}请指定版本号，例如: ./build.sh release 1.0.1${NC}"
        exit 1
    fi

    local tag="v${version}"

    if git tag -l | grep -q "^${tag}$"; then
        echo -e "${RED}tag ${tag} 已存在，请使用新版本号${NC}"
        echo "已有 tags:"
        git tag -l "v*" | sort -V
        exit 1
    fi

    echo -e "${YELLOW}检查工作区...${NC}"
    if [ -n "$(git status --porcelain)" ]; then
        echo -e "${YELLOW}有未提交的修改，先提交...${NC}"
        git add -A
        git commit -m "chore: release ${tag}"
    fi

    echo -e "${YELLOW}推送代码到 origin/main...${NC}"
    git push origin main

    echo -e "${YELLOW}创建 tag: ${tag}${NC}"
    git tag -a "$tag" -m "Release ${tag}"

    echo -e "${YELLOW}推送 tag 到 GitHub...${NC}"
    git push origin "$tag"

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN} tag ${tag} 已推送，GitHub Actions 开始编译${NC}"
    echo -e "${GREEN} Linux + Windows 跨平台构建中...${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "查看构建进度: https://github.com/gengshenchen/esim-desktop/actions"
    echo "构建完成后到 Releases 页面下载: https://github.com/gengshenchen/esim-desktop/releases"
}

cmd_release_list() {
    echo "已有的 release tags:"
    git tag -l "v*" | sort -V
    echo ""
    echo "远程 tags:"
    git ls-remote --tags origin 2>/dev/null | grep "refs/tags/v" | awk '{print $2}' | sed 's|refs/tags/||'
}

case "${1:-}" in
    dev)        cmd_dev ;;
    build)      cmd_build ;;
    clean)      cmd_clean ;;
    rebuild)    cmd_rebuild ;;
    check)      cmd_check ;;
    release)    cmd_release "$2" ;;
    release-list) cmd_release_list ;;
    -h|--help|help|"") usage ;;
    *)
        echo -e "${RED}未知命令: $1${NC}"
        usage
        ;;
esac
