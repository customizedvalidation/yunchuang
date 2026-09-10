import React, { useState, useEffect } from 'react';
import { useLoginMutation } from '../store/api';
import { useNavigate } from 'react-router-dom';
import { Form, Input, Button, App, Alert } from 'antd';
import {
  LockOutlined,
  UserOutlined,
  CloudServerOutlined,
  ThunderboltOutlined,
  SafetyCertificateOutlined,
  RocketOutlined,
} from '@ant-design/icons';
import { useThemeMode } from '../theme/ThemeModeContext';
import { getNeutral } from '../theme/tokens';
import './Login.css';

/** 从 RTK Query 的错误对象中取出可读的错误信息 */
type ApiErrorData = { message?: string; data?: { message?: string } };
function apiErrorMessage(err: unknown): string {
  const data = (err as { data?: ApiErrorData } | null)?.data;
  return data?.data?.message || data?.message || '请检查用户名和密码';
}

/** 科技蓝设计令牌 */
const TECH_BLUE = {
  primary: '#1677ff',
  primaryDark: '#0958d9',
  primaryLight: '#4096ff',
  primaryBg: 'rgba(22, 119, 255, 0.1)',
  primaryBorder: 'rgba(22, 119, 255, 0.3)',
  gradient: 'linear-gradient(135deg, #1677ff 0%, #0958d9 100%)',
  gradientHover: 'linear-gradient(135deg, #4096ff 0%, #1677ff 100%)',
  glow: '0 0 40px rgba(22, 119, 255, 0.3)',
  glowStrong: '0 0 60px rgba(22, 119, 255, 0.5)',
};

/** 平台特性列表 */
const FEATURES = [
  {
    icon: <CloudServerOutlined />,
    title: '多集群统一调度',
    desc: '跨集群资源池化管理，支持 K8s / Slurm / LSF 混合调度',
  },
  {
    icon: <ThunderboltOutlined />,
    title: 'GPU 细粒度分配',
    desc: '支持 1/2、1/4 GPU 切分与显存超发，资源利用率提升 30%+',
  },
  {
    icon: <SafetyCertificateOutlined />,
    title: '企业级安全防护',
    desc: 'RBAC 权限体系 + 多租户隔离 + 审计日志，合规无忧',
  },
];

const Login: React.FC = () => {
  const { message } = App.useApp();
  const [login, { isLoading, error }] = useLoginMutation();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const [loginAttempts, setLoginAttempts] = useState(0);
  const [isLocked, setIsLocked] = useState(false);
  const [lockTime, setLockTime] = useState(0);

  const { mode } = useThemeMode();
  const neutral = getNeutral(mode);
  const isDark = mode === 'dark';

  // 右侧登录区背景
  const rightBg = isDark
    ? 'linear-gradient(135deg, #0d1b2a 0%, #112240 50%, #0a1628 100%)'
    : 'linear-gradient(135deg, #f0f5ff 0%, #f8faff 50%, #eef4ff 100%)';

  // 登录卡片背景
  const cardBg = isDark ? 'rgba(13, 27, 42, 0.85)' : 'rgba(255, 255, 255, 0.9)';
  const cardBorder = isDark ? 'rgba(22, 119, 255, 0.2)' : 'rgba(22, 119, 255, 0.15)';
  const inputBg = isDark ? 'rgba(15, 23, 42, 0.6)' : 'rgba(255, 255, 255, 0.95)';
  const inputBorder = isDark ? 'rgba(148, 163, 184, 0.2)' : 'rgba(22, 119, 255, 0.2)';

  useEffect(() => {
    if (isLocked) {
      const timer = setTimeout(() => {
        setIsLocked(false);
        setLoginAttempts(0);
      }, lockTime);
      return () => clearTimeout(timer);
    }
  }, [isLocked, lockTime]);

  const onFinish = async (values: { username: string; password: string }) => {
    if (isLocked) {
      message.error('登录失败次数过多，请稍后再试');
      return;
    }

    try {
      const result = await login(values).unwrap();
      const { user, expires_at } = result.data;
      localStorage.setItem(
        'user',
        JSON.stringify({
          username: user?.username ?? '',
          email: user?.email ?? '',
          role: user?.role ?? '',
        }),
      );
      localStorage.setItem('auth_expiry', String((expires_at ?? 0) * 1000));
      message.success('登录成功');
      setLoginAttempts(0);
      navigate('/dashboard');
    } catch (err) {
      const newAttempts = loginAttempts + 1;
      setLoginAttempts(newAttempts);

      if (newAttempts >= 5) {
        setIsLocked(true);
        setLockTime(60000);
        message.error('登录失败次数过多，账号已被锁定1分钟');
      } else {
        message.error(apiErrorMessage(err));
      }
    }
  };

  return (
    <main
      aria-label="登录"
      className="login-page"
      style={{
        minHeight: '100vh',
        display: 'flex',
        background: rightBg,
        position: 'relative',
        overflow: 'hidden',
      }}
    >
      {/* 左侧品牌展示区 - 桌面端可见 */}
      <section className="login-brand-section" aria-hidden="true">
        {/* 科技网格背景 */}
        <div className="login-grid-bg" />

        {/* 光晕装饰 */}
        <div
          className="login-glow login-glow-1"
          style={{
            position: 'absolute',
            top: '-10%',
            right: '-20%',
            width: '500px',
            height: '500px',
            background: 'radial-gradient(circle, rgba(22,119,255,0.25) 0%, transparent 60%)',
            borderRadius: '50%',
            animation: 'float 15s ease-in-out infinite',
          }}
        />
        <div
          className="login-glow login-glow-2"
          style={{
            position: 'absolute',
            bottom: '-15%',
            left: '-10%',
            width: '400px',
            height: '400px',
            background: 'radial-gradient(circle, rgba(64,150,255,0.18) 0%, transparent 60%)',
            borderRadius: '50%',
            animation: 'float 12s ease-in-out infinite reverse',
          }}
        />

        {/* 品牌内容 */}
        <div className="login-brand-content">
          {/* Logo */}
          <div className="login-brand-logo">
            <div
              style={{
                width: '56px',
                height: '56px',
                background: TECH_BLUE.gradient,
                borderRadius: '16px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                boxShadow: TECH_BLUE.glow,
              }}
            >
              <RocketOutlined style={{ fontSize: '28px', color: '#fff' }} />
            </div>
            <div>
              <div
                style={{
                  fontSize: '22px',
                  fontWeight: 700,
                  color: '#fff',
                  letterSpacing: '0.5px',
                }}
              >
                Metaclouds
              </div>
              <div style={{ fontSize: '13px', color: 'rgba(255,255,255,0.6)' }}>
                企业级算力调度平台
              </div>
            </div>
          </div>

          {/* 主标题 */}
          <h1 className="login-brand-title">
            智能算力调度
            <br />
            <span style={{ background: TECH_BLUE.gradient, WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
              驱动 AI 创新
            </span>
          </h1>

          <p className="login-brand-subtitle">
            统一管理多集群 GPU 资源，提供从训练到推理的全链路算力服务，
            让每一次计算都高效、稳定、安全。
          </p>

          {/* 特性列表 */}
          <div className="login-features">
            {FEATURES.map((feature, index) => (
              <div key={index} className="login-feature-item">
                <div className="login-feature-icon">{feature.icon}</div>
                <div>
                  <div className="login-feature-title">{feature.title}</div>
                  <div className="login-feature-desc">{feature.desc}</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* 底部版权 */}
        <div className="login-brand-footer">
          <span>© 2026 Metaclouds. All rights reserved.</span>
          <span>Version 2.0.0</span>
        </div>
      </section>

      {/* 右侧登录表单区 */}
      <section className="login-form-section">
        {/* 移动端 Logo（仅小屏显示） */}
        <div className="login-mobile-logo">
          <div
            style={{
              width: '48px',
              height: '48px',
              background: TECH_BLUE.gradient,
              borderRadius: '14px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: TECH_BLUE.glow,
              margin: '0 auto 12px',
            }}
          >
            <RocketOutlined style={{ fontSize: '24px', color: '#fff' }} />
          </div>
          <div style={{ fontSize: '20px', fontWeight: 700, color: neutral.text1 }}>
            Metaclouds
          </div>
        </div>

        {/* 登录卡片 */}
        <div
          className="login-card-wrapper"
          style={{
            width: '100%',
            maxWidth: '420px',
            background: cardBg,
            backdropFilter: 'blur(24px)',
            WebkitBackdropFilter: 'blur(24px)',
            border: `1px solid ${cardBorder}`,
            borderRadius: '24px',
            boxShadow: `0 25px 60px rgba(0, 0, 0, 0.2), ${TECH_BLUE.glow}`,
            padding: '40px 36px',
            position: 'relative',
            overflow: 'hidden',
          }}
        >
          {/* 卡片顶部光带 */}
          <div
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              right: 0,
              height: '3px',
              background: TECH_BLUE.gradient,
            }}
          />

          {/* 标题区 */}
          <div style={{ textAlign: 'center', marginBottom: '32px' }}>
            <h2
              style={{
                fontSize: '26px',
                fontWeight: 700,
                color: neutral.text1,
                margin: '0 0 8px 0',
              }}
            >
              欢迎回来
            </h2>
            <p style={{ color: neutral.text3, fontSize: '14px', margin: 0 }}>
              登录您的账户以继续使用算力调度平台
            </p>
          </div>

          {/* 错误提示 */}
          {error && (
            <Alert
              message="登录失败"
              description={apiErrorMessage(error)}
              type="error"
              showIcon
              style={{ marginBottom: '20px', borderRadius: '10px' }}
            />
          )}
          {isLocked && (
            <Alert
              message="账号已锁定"
              description="登录失败次数过多，请1分钟后再试"
              type="error"
              showIcon
              style={{ marginBottom: '20px', borderRadius: '10px' }}
            />
          )}

          {/* 登录表单 */}
          <Form form={form} onFinish={onFinish} layout="vertical" requiredMark={false}>
            <Form.Item
              name="username"
              label={<span style={{ fontWeight: 500, color: neutral.text2 }}>用户名</span>}
              rules={[
                { required: true, message: '请输入用户名' },
                { min: 3, max: 20, message: '用户名长度应在3-20个字符之间' },
              ]}
              style={{ marginBottom: '20px' }}
            >
              <Input
                placeholder="请输入用户名"
                disabled={isLoading || isLocked}
                prefix={<UserOutlined style={{ color: TECH_BLUE.primary }} />}
                autoComplete="username"
                style={{
                  background: inputBg,
                  border: `1px solid ${inputBorder}`,
                  borderRadius: '12px',
                  color: neutral.text1,
                  height: '46px',
                  fontSize: '14px',
                  transition: 'all 0.3s ease',
                }}
              />
            </Form.Item>

            <Form.Item
              name="password"
              label={<span style={{ fontWeight: 500, color: neutral.text2 }}>密码</span>}
              rules={[
                { required: true, message: '请输入密码' },
                { min: 6, message: '密码长度至少6个字符' },
              ]}
              style={{ marginBottom: '28px' }}
            >
              <Input.Password
                placeholder="请输入密码"
                disabled={isLoading || isLocked}
                prefix={<LockOutlined style={{ color: TECH_BLUE.primary }} />}
                autoComplete="current-password"
                style={{
                  background: inputBg,
                  border: `1px solid ${inputBorder}`,
                  borderRadius: '12px',
                  color: neutral.text1,
                  height: '46px',
                  fontSize: '14px',
                  transition: 'all 0.3s ease',
                }}
              />
            </Form.Item>

            <Form.Item style={{ marginBottom: '0' }}>
              <Button
                type="primary"
                htmlType="submit"
                loading={isLoading}
                block
                disabled={isLoading || isLocked}
                className="login-submit-btn"
                style={{
                  height: '50px',
                  borderRadius: '12px',
                  fontSize: '16px',
                  fontWeight: 600,
                  background: TECH_BLUE.gradient,
                  border: 'none',
                  boxShadow: '0 6px 20px rgba(22, 119, 255, 0.4)',
                  letterSpacing: '2px',
                  transition: 'all 0.3s ease',
                }}
              >
                {isLoading ? '登录中...' : '登 录'}
              </Button>
            </Form.Item>
          </Form>

          {/* 底部提示 */}
          <div
            style={{
              marginTop: '28px',
              paddingTop: '20px',
              borderTop: `1px solid ${cardBorder}`,
              textAlign: 'center',
            }}
          >
            <p style={{ color: neutral.text3, fontSize: '13px', margin: 0, lineHeight: 1.6 }}>
              默认账号：<span style={{ color: TECH_BLUE.primary, fontWeight: 500 }}>admin</span>
              <br />
              初始密码由部署配置决定，请联系管理员获取
            </p>
          </div>
        </div>

        {/* 移动端版权 */}
        <div className="login-mobile-footer">
          <span>© 2026 Metaclouds. All rights reserved.</span>
        </div>
      </section>
    </main>
  );
};

export default Login;
