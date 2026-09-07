/**
 * CSS 模块代理 mock
 *
 * 替代 identity-obj-proxy（离线环境未安装）。
 * 导入 CSS 文件时返回空对象，组件中的 className 访问返回 undefined，
 * 不影响测试逻辑（测试关注用户可见行为，不关注具体类名）。
 *
 * 安装 identity-obj-proxy 后，可在 jest.config.js 中将 moduleNameMapper
 * 的 CSS 规则改为 'identity-obj-proxy' 以获得更真实的类名代理。
 */
module.exports = {};
