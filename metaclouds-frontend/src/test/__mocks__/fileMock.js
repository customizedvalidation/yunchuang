/**
 * 静态资源（图片/字体等）代理 mock
 *
 * 导入图片或字体文件时返回文件名占位符，避免 Jest 解析二进制文件报错。
 */
module.exports = 'test-file-stub';
