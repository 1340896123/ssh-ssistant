# Tasks
- [x] Task 1: 重构主工作区布局控制，突出终端主任务焦点。
  - [x] SubTask 1.1: 设计并实现文件区、AI 区的快速收起与恢复入口
  - [x] SubTask 1.2: 提供主工作区预设布局或显隐模式切换
  - [x] SubTask 1.3: 校验布局切换后会话状态、尺寸缓存和交互连续性

- [x] Task 2: 优化连接区和会话区的可发现性与操作效率。
  - [x] SubTask 2.1: 重做连接区空状态，提供新建、导入、创建分组等明确入口
  - [x] SubTask 2.2: 增强连接区高频入口，如搜索、最近使用或收藏的承载位
  - [x] SubTask 2.3: 优化会话标签信息密度与批量操作能力

- [x] Task 3: 优化文件管理器与终端增强能力的显性入口。
  - [x] SubTask 3.1: 明确文件联动动作，如发送到终端、加入 AI 上下文、在编辑器打开
  - [x] SubTask 3.2: 优化路径导航、多选反馈和分页加载提示
  - [x] SubTask 3.3: 为终端搜索、清屏、重连、AI 补全提供更直观入口或提示

- [x] Task 4: 改进 AI 助手、设置面板和全局文案一致性。
  - [x] SubTask 4.1: 调整 AI 助手初始状态与欢迎引导，使其贴合当前会话上下文
  - [x] SubTask 4.2: 重新梳理设置面板的信息架构与危险操作分区
  - [x] SubTask 4.3: 统一关键界面的本地化文案、空状态与状态反馈

- [x] Task 5: 验证界面体验优化结果。
  - [x] SubTask 5.1: 运行前端构建或类型检查，确认界面改动未引入错误
  - [x] SubTask 5.2: 手动验证主要工作流，包括连接、切换布局、文件操作、终端增强和 AI 联动

# Task Dependencies
- Task 2 depends on Task 1 only for需要复用布局入口时的样式与交互约定
- Task 3 depends on Task 1 because 文件区与终端区的交互入口需适配新布局
- Task 4 can proceed in parallel with Task 2 and Task 3 after Task 1 defines整体交互基线
- Task 5 depends on Task 1, Task 2, Task 3, and Task 4
