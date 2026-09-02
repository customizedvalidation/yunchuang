import React, { useState, useEffect } from 'react';
import { useLoginMutation } from '../store/api';
import { useNavigate } from 'react-router-dom';
import { Form, Input, Button, Card, App, Alert } from 'antd';
import { LockOutlined, UserOutlined, EyeOutlined } from '@ant-design/icons';
import { useThemeMode } from '../theme/ThemeModeContext';
import { getNeutral, brand } from '../theme/tokens';
import './Login.css';

/** 从 RTK Query 的错误对象中取出可读的错误信息 */
type ApiErrorData = { message?: string; data?: { message?: string } };
function apiErrorMessage(err: unknown): string {
  const data = (err as { data?: ApiErrorData } | null)?.data;
  return data?.data?.message || data?.message || '请检查用户名和密码';
}

const Login: React.FC = () => {
  const { message } = App.useApp();
  const [login, { isLoading, error }] = useLoginMutation();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const [loginAttempts, setLoginAttempts] = useState(0);
  const [isLocked, setIsLocked] = useState(false);
  const [lockTime, setLockTime] = useState(0);
  const [, setShowPassword] = useState(false);

  const { mode } = useThemeMode();
  const neutral = getNeutral(mode);
  const isDark = mode === 'dark';
  const pageBg = isDark
    ? 'linear-gradient(135deg, #0a1120 0%, #111a2b 50%, #162135 100%)'
    : 'linear-gradient(135deg, #eef4ff 0%, #f5f7fc 50%, #eaf0fa 100%)';
  const cardBg = isDark ? 'rgba(17,26,43,0.85)' : 'rgba(255,255,255,0.85)';
  const cardBorder = isDark ? 'rgba(255,255,255,0.10)' : 'rgba(14,23,38,0.08)';
  const inputBg = isDark ? 'rgba(15,23,42,0.6)' : 'rgba(255,255,255,0.9)';
  const inputBorder = isDark ? 'rgba(148,163,184,0.20)' : 'rgba(14,23,38,0.12)';
  const brandGradient = `linear-gradient(135deg, ${brand[500]} 0%, ${brand[600]} 100%)`;

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
      localStorage.setItem('token', result.data.token);
      // 登录响应只回传 token，用户信息需由 token 解析后落盘，
      // 否则顶栏只能回退到硬编码的 'admin'，刷新后也无法还原真实身份。
      try {
        const b64 = result.data.token
          .split('.')[1]
          .replace(/-/g, '+')
          .replace(/_/g, '/');
        const payload = JSON.parse(atob(b64));
        localStorage.setItem(
          'user',
          JSON.stringify({
            username: payload.username || payload.email || '',
            email: payload.email || '',
            role: payload.role || '',
          }),
        );
      } catch {
        // token 解析失败不应影响登录主流程
      }
      message.success('登录成功');
      setLoginAttempts(0);
      navigate('/dashboard');
    } catch (error) {
      const newAttempts = loginAttempts + 1;
      setLoginAttempts(newAttempts);

      if (newAttempts >= 5) {
        setIsLocked(true);
        setLockTime(60000);
        message.error('登录失败次数过多，账号已被锁定1分钟');
      } else {
        message.error(apiErrorMessage(error));
      }
    }
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: pageBg,
        position: 'relative',
        overflow: 'hidden',
      }}
    >
      <div
        className="float-animation"
        style={{
          position: 'absolute',
          top: -200,
          right: -200,
          width: 600,
          height: 600,
          background: 'radial-gradient(circle, rgba(47,107,255,0.20) 0%, transparent 60%)',
          borderRadius: '50%',
          animation: 'float 15s ease-in-out infinite',
        }}
      />
      <div
        className="float-animation"
        style={{
          position: 'absolute',
          bottom: -150,
          left: -150,
          width: 500,
          height: 500,
          background: 'radial-gradient(circle, rgba(124,92,255,0.14) 0%, transparent 60%)',
          borderRadius: '50%',
          animation: 'float 12s ease-in-out infinite reverse',
        }}
      />
      <div
        className="float-animation"
        style={{
          position: 'absolute',
          top: '30%',
          left: '20%',
          width: 300,
          height: 300,
          background: 'radial-gradient(circle, rgba(0,184,169,0.10) 0%, transparent 50%)',
          borderRadius: '50%',
          animation: 'float 18s ease-in-out infinite',
        }}
      />

      <Card
        className="login-card"
        style={{
          width: 'clamp(320px, 92vw, 440px)',
          background: cardBg,
          backdropFilter: 'blur(20px)',
          border: `1px solid ${cardBorder}`,
          borderRadius: '20px',
          boxShadow: '0 25px 50px rgba(0, 0, 0, 0.25), 0 0 0 1px rgba(255, 255, 255, 0.04)',
          position: 'relative',
          zIndex: 1,
        }}
      >
        <div style={{ textAlign: 'center', marginBottom: '28px' }}>
          <div
            style={{
              width: '72px',
              height: '72px',
              margin: '0 auto 16px',
              background: brandGradient,
              borderRadius: '20px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: '0 8px 30px rgba(47, 107, 255, 0.4)',
            }}
          >
            <LockOutlined style={{ fontSize: '32px', color: '#fff' }} />
          </div>
          <h1
            style={{
              fontSize: '24px',
              fontWeight: 600,
              color: neutral.text1,
              margin: '0 0 8px 0',
            }}
          >
            <span style={{ background: brandGradient, WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
              Metaclouds
            </span>
          </h1>
          <p style={{ color: neutral.text3, fontSize: '14px', margin: 0 }}>算力调度平台</p>
        </div>

        {error && (
          <Alert
            message="登录失败"
            description={apiErrorMessage(error)}
            type="error"
            showIcon
            style={{ marginBottom: '20px', background: 'var(--mc-danger-soft)', borderColor: 'var(--mc-danger-border)' }}
          />
        )}
        {isLocked && (
          <Alert
            message="账号已锁定"
            description="登录失败次数过多，请1分钟后再试"
            type="error"
            showIcon
            style={{ marginBottom: '20px', background: 'var(--mc-danger-soft)', borderColor: 'var(--mc-danger-border)' }}
          />
        )}

        <Form form={form} onFinish={onFinish} layout="vertical">
          <Form.Item
            name="username"
            label="用户名"
            rules={[
              { required: true, message: '请输入用户名' },
              { min: 3, max: 20, message: '用户名长度应在3-20个字符之间' },
            ]}
          >
            <Input
              placeholder="请输入用户名"
              disabled={isLoading || isLocked}
              prefix={<UserOutlined style={{ color: neutral.text3 }} />}
              style={{
                background: inputBg,
                border: `1px solid ${inputBorder}`,
                borderRadius: '12px',
                color: neutral.text1,
                height: '44px',
              }}
            />
          </Form.Item>
          <Form.Item
            name="password"
            label="密码"
            rules={[
              { required: true, message: '请输入密码' },
              { min: 6, message: '密码长度至少6个字符' },
            ]}
          >
            <Input.Password
              placeholder="请输入密码"
              disabled={isLoading || isLocked}
              prefix={<LockOutlined style={{ color: neutral.text3 }} />}
              iconRender={(visible) => (
                <EyeOutlined
                  style={{ color: neutral.text3, cursor: 'pointer' }}
                  onClick={() => setShowPassword(!visible)}
                />
              )}
              style={{
                background: inputBg,
                border: `1px solid ${inputBorder}`,
                borderRadius: '12px',
                color: neutral.text1,
                height: '44px',
              }}
            />
          </Form.Item>
          <Form.Item>
            <Button
              type="primary"
              htmlType="submit"
              loading={isLoading}
              block
              disabled={isLoading || isLocked}
              style={{
                height: '48px',
                borderRadius: '12px',
                fontSize: '15px',
                fontWeight: 500,
                background: brandGradient,
                border: 'none',
                boxShadow: '0 4px 14px rgba(47, 107, 255, 0.4)',
              }}
            >
              {isLoading ? '登录中...' : '登 录'}
            </Button>
          </Form.Item>
        </Form>

        <div
          style={{
            marginTop: '20px',
            paddingTop: '20px',
            borderTop: `1px solid ${cardBorder}`,
            textAlign: 'center',
          }}
        >
          <p style={{ color: neutral.text3, fontSize: '13px', margin: 0 }}>
            默认账号：<span style={{ color: neutral.text2 }}>admin</span>（初始密码由部署配置决定，请联系管理员获取）
          </p>
        </div>
      </Card>

    </div>
  );
};

export default Login;
