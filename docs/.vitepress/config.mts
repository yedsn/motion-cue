import { defineConfig } from "vitepress";

const githubRepo = "https://github.com/yedsn/motion-cue";
const repoName = process.env.GITHUB_REPOSITORY?.split("/")[1] ?? "motion-cue";
const base = process.env.DOCS_BASE ?? (process.env.GITHUB_ACTIONS === "true" ? `/${repoName}/` : "/");

export default defineConfig({
  title: "MotionCue",
  description: "Windows 优先的全屏动画与音效触发器。",
  lang: "zh-CN",
  base,
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: `${base}logo.svg` }],
    ["meta", { name: "theme-color", content: "#121915" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:title", content: "MotionCue" }],
    ["meta", { property: "og:description", content: "用动画和音效为开发工作流提供即时反馈。" }],
  ],
  themeConfig: {
    logo: "/logo.svg",
    nav: [
      { text: "首页", link: "/" },
      { text: "快速开始", link: "/guide/quick-start" },
      { text: "开发文档", link: "/develop/setup" },
      { text: "GitHub", link: githubRepo },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "使用说明",
          items: [
            { text: "快速开始", link: "/guide/quick-start" },
            { text: "调用方式", link: "/guide/invocation" },
            { text: "内置动画", link: "/guide/built-in-cues" },
          ],
        },
        {
          text: "扩展能力",
          items: [{ text: "Web 插件格式", link: "/guide/plugin-format" }],
        },
      ],
      "/develop/": [
        {
          text: "开发说明",
          items: [
            { text: "开发环境", link: "/develop/setup" },
            { text: "构建与验证", link: "/develop/build-and-verify" },
            { text: "GitHub 部署", link: "/develop/github-pages" },
            { text: "GitHub Release", link: "/develop/release" },
          ],
        },
      ],
    },
    socialLinks: [{ icon: "github", link: githubRepo }],
    search: { provider: "local" },
    editLink: {
      pattern: `${githubRepo}/edit/main/docs/:path`,
      text: "在 GitHub 上编辑此页",
    },
    outline: { label: "页面导航" },
    docFooter: { prev: "上一页", next: "下一页" },
    lastUpdated: { text: "最后更新于" },
    footer: {
      message: "MotionCue is released under the MIT License.",
      copyright: "Copyright 2026 MotionCue Contributors",
    },
  },
});
