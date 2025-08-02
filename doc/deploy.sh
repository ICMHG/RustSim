#!/bin/bash

# RustSim 算法文档部署脚本
# 用于构建GitBook并部署到GitHub Pages

set -e

echo "🚀 开始构建RustSim算法文档..."

# 检查Node.js版本
check_node_version() {
    local node_version=$(node --version 2>/dev/null | sed 's/v//')
    if [ -z "$node_version" ]; then
        echo "❌ Node.js未安装"
        exit 1
    fi
    
    echo "✅ 使用Node.js v$node_version"
}

# 构建文档
build_docs() {
    echo "🔨 构建文档..."
    
    # 清理之前的构建
    echo "🧹 清理之前的构建..."
    rm -rf _book
    
    # 使用npx gitbook-cli构建
    echo "📦 使用GitBook构建..."
    if npx gitbook-cli build . _book; then
        echo "✅ GitBook构建成功"
        
        # 清理多余文件
        echo "🧹 清理多余文件..."
        rm -f _book/deploy.sh _book/prompt.md 2>/dev/null || true
        
        return 0
    else
        echo "❌ GitBook构建失败"
        exit 1
    fi
}

# 部署到GitHub Pages
deploy_to_github() {
    if [ "$1" = "deploy" ]; then
        echo "🚀 部署到GitHub Pages..."
        
        # 检查Git仓库位置
        local git_dir=""
        if [ -d ".git" ]; then
            git_dir="."
        elif [ -d "../.git" ]; then
            git_dir=".."
        else
            echo "❌ 当前目录及其上级目录都不是Git仓库"
            exit 1
        fi
        
        echo "📁 使用Git仓库: $git_dir"
        
        # 切换到Git仓库目录
        cd "$git_dir"
        
        # 复制文档到临时目录
        echo "📋 准备文档..."
        if [ "$git_dir" = "." ]; then
            cp -r _book ./_book_temp
        else
            cp -r doc/_book ./_book_temp
        fi
        
        # 切换到gh-pages分支
        echo "📝 切换到gh-pages分支..."
        git checkout gh-pages
        
        # 清理当前分支内容
        git rm -rf . || true
        
        # 复制文档内容
        echo "📋 复制文档内容..."
        cp -r _book_temp/* .
        rm -rf _book_temp
        
        # 提交更改
        echo "📤 提交更改..."
        git add .
        git commit -m "Update documentation $(date)" || {
            echo "⚠️  没有新的更改需要提交"
        }
        
        # 推送到远程仓库
        echo "🚀 推送到GitHub..."
        git push origin gh-pages
        
        # 切换回main分支
        git checkout main
        
        # 返回到doc目录
        if [ "$git_dir" = ".." ]; then
            cd doc
        fi
        
        echo "✅ 部署完成！"
        echo "🌐 文档将在几分钟后可在以下地址访问："
        echo "   https://icmhg.github.io/RustSim/"
    else
        echo "📖 文档已构建完成，位于 _book/ 目录"
        echo "💡 运行 './deploy.sh deploy' 来部署到GitHub Pages"
    fi
}

# 主执行流程
main() {
    # 检查Node.js版本
    check_node_version
    
    # 构建文档
    build_docs
    
    # 检查构建是否成功
    if [ ! -d "_book" ]; then
        echo "❌ 构建失败：_book目录不存在"
        exit 1
    fi
    
    echo "✅ 文档构建成功！"
    
    # 部署
    deploy_to_github "$1"
    
    echo "🎉 完成！"
}

# 错误处理
trap 'echo "❌ 脚本执行被中断"; exit 1' INT TERM

# 执行主函数
main "$@" 