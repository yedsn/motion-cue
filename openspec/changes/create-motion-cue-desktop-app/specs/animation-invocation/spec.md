## Purpose

定义外部应用、Hook 与脚本如何通过稳定、安全且可诊断的入口触发 MotionCue 动画，并确保未运行、重复启动和错误输入时仍有确定行为。

## ADDED Requirements

### Requirement: 支持自定义链接调用动画
系统 SHALL 注册 `motioncue://` 自定义链接协议，并支持通过 `motioncue://play/<command>` 或 `motioncue://play?command=<command>` 触发已启用动画。

#### Scenario: 通过路径调用已注册命令
- **WHEN** 外部应用打开 `motioncue://play/task-complete`
- **THEN** 系统解析 `task-complete` 并提交对应动画播放请求

#### Scenario: 应用未运行时调用自定义链接
- **WHEN** 用户在 MotionCue 未运行时打开有效的 `motioncue://play/confetti` 链接
- **THEN** 系统启动 MotionCue、完成单实例初始化并播放 `confetti` 动画

#### Scenario: 自定义链接命令不存在
- **WHEN** 外部应用打开一个命令不存在、已禁用或无法加载的自定义链接
- **THEN** 系统不播放其他动画，并在本地诊断记录中保存可理解的失败原因

### Requirement: 支持命令行调用动画
系统 SHALL 提供 `motion-cue play <command>` 命令行入口，并为自动化调用返回确定的退出状态和机器可读错误信息。

#### Scenario: 命令行成功触发动画
- **WHEN** Hook 执行 `motion-cue play success --text "发布完成"`
- **THEN** 命令将请求转发给运行实例并以成功状态退出

#### Scenario: 命令行调用未知命令
- **WHEN** Hook 执行 `motion-cue play missing-animation`
- **THEN** 命令不触发动画、向标准错误输出未知命令信息并以非零状态退出

#### Scenario: 查询可用命令
- **WHEN** 调用方执行 `motion-cue list --json`
- **THEN** 系统输出所有已启用动画的主命令、别名和可传参数，且输出为有效 JSON

### Requirement: 采用单实例命令转发
系统 SHALL 只保留一个常驻 MotionCue 实例；后续由自定义链接、命令行或桌面快捷方式发起的进程 SHALL 将请求转发给已有实例后退出。

#### Scenario: 已有实例时再次调用
- **WHEN** MotionCue 已在托盘运行且新进程收到播放请求
- **THEN** 新进程将完整请求安全转发给已有实例，不创建第二套托盘图标或覆盖窗口

#### Scenario: 转发暂时失败
- **WHEN** 新进程无法在规定时间内连接已有实例
- **THEN** 命令行调用以非零状态退出，自定义链接调用写入诊断记录，系统不得静默启动相互竞争的第二实例

### Requirement: 验证命令和播放参数
系统 MUST 在进入动画渲染器之前验证所有外部输入。命令 SHALL 使用不区分大小写的规范化匹配，并限制为字母、数字、短横线和下划线；文本、颜色、时长及插件声明参数 SHALL 遵守类型和长度限制。

#### Scenario: 传入合法覆盖参数
- **WHEN** 调用方为允许文本参数的动画传入合法 `text`、`color` 和 `duration` 值
- **THEN** 系统将规范化后的值传给动画，并保留动画配置中未被覆盖的默认值

#### Scenario: 参数超过限制
- **WHEN** 调用方传入超长文本、无效颜色、超出允许范围的时长或插件未声明的参数
- **THEN** 系统拒绝该请求并返回或记录具体校验错误，不将原始值直接交给渲染器

#### Scenario: 命令大小写不同
- **WHEN** 调用方触发 `TASK-COMPLETE` 而目录中的规范命令为 `task-complete`
- **THEN** 系统将两者视为同一命令并只解析到一个动画

### Requirement: 提供基础控制命令
系统 SHALL 提供列出动画、停止当前动画和打开管理界面的外部控制能力，且控制命令不得绕过正常权限边界。

#### Scenario: 停止当前动画
- **WHEN** 调用方执行 `motion-cue stop`
- **THEN** 系统停止当前视觉效果和音效、清理播放会话并隐藏覆盖窗口

#### Scenario: 打开管理界面
- **WHEN** 调用方执行 `motion-cue open`
- **THEN** 系统显示并聚焦 MotionCue 管理界面，而不启动重复实例

