import React, { useState, useEffect } from 'react';
import { useLoginMutation } from '../store/api';
import { useNavigate } from 'react-router-dom';
import { Form, Input, Button, Card, message, Alert } from 'antd';
import { LockOutlined, UserOutlined, EyeOutlined } from '@ant-design/icons';

const Login: React.FC = () => {
  const [login, { isLoading, error }] = useLoginMutation();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const [loginAttempts, setLoginAttempts] = useState(0);
  const [isLocked, setIsLocked] = useState(false);
  const [lockTime, setLockTime] = useState(0);
  const [, setShowPassword] = useState(false);

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
      message.success('登录成功');
      setLoginAttempts(0);
      navigate('/dashboard');
    } catch (error: any) {
      const newAttempts = loginAttempts + 1;
      setLoginAttempts(newAttempts);
      
      if (newAttempts >= 5) {
        setIsLocked(true);
        setLockTime(60000);
        message.error('登录失败次数过多，账号已被锁定1分钟');
      } else {
        message.error(error.data?.data?.message || error.data?.message || '登录失败，请检查用户名和密码');
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
        background: 'linear-gradient(135deg, #0f172a 0%, #1e293b 50%, #334155 100%)',
        position: 'relative',
        overflow: 'hidden'
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
          background: 'radial-gradient(circle, rgba(59, 130, 246, 0.2) 0%, transparent 60%)',
          borderRadius: '50%',
          animation: 'float 15s ease-in-out infinite'
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
          background: 'radial-gradient(circle, rgba(139, 92, 246, 0.15) 0%, transparent 60%)',
          borderRadius: '50%',
          animation: 'float 12s ease-in-out infinite reverse'
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
          background: 'radial-gradient(circle, rgba(16, 185, 129, 0.1) 0%, transparent 50%)',
          borderRadius: '50%',
          animation: 'float 18s ease-in-out infinite'
        }}
      />
      
      <Card 
        className="login-card"
        style={{ 
          width: 420,
          background: 'rgba(30, 41, 59, 0.85)',
          backdropFilter: 'blur(20px)',
          border: '1px solid rgba(255, 255, 255, 0.1)',
          borderRadius: '20px',
          boxShadow: '0 25px 50px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(255, 255, 255, 0.05)',
          position: 'relative',
          zIndex: 1
        }}
        styles={{ body: { padding: '32px' } }}
      >
        <div style={{ textAlign: 'center', marginBottom: '28px' }}>
          <div 
            style={{ 
              width: '72px', 
              height: '72px',
              margin: '0 auto 16px',
              background: 'linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%)',
              borderRadius: '20px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: '0 8px 30px rgba(59, 130, 246, 0.4)'
            }}
          >
            <LockOutlined style={{ fontSize: '32px', color: '#fff' }} />
          </div>
          <h1 
            style={{ 
              fontSize: '24px', 
              fontWeight: 600, 
              color: '#f1f5f9',
              margin: '0 0 8px 0'
            }}
          >
            <span style={{ background: 'linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
              Metaclouds
            </span>
          </h1>
          <p style={{ color: '#94a3b8', fontSize: '14px', margin: 0 }}>
            算力调度平台
          </p>
        </div>

        {error && (
          <Alert
            message="登录失败"
            description={(error as any).data?.message || '请检查用户名和密码'}
            type="error"
            showIcon
            style={{ marginBottom: '20px', background: 'rgba(239, 68, 68, 0.1)', borderColor: 'rgba(239, 68, 68, 0.3)' }}
          />
        )}
        {isLocked && (
          <Alert
            message="账号已锁定"
            description="登录失败次数过多，请1分钟后再试"
            type="error"
            showIcon
            style={{ marginBottom: '20px', background: 'rgba(239, 68, 68, 0.1)', borderColor: 'rgba(239, 68, 68, 0.3)' }}
          />
        )}

        <Form
          form={form}
          onFinish={onFinish}
          layout="vertical"
        >
          <Form.Item
            name="username"
            label="用户名"
            rules={[
              { required: true, message: '请输入用户名' },
              { min: 3, max: 20, message: '用户名长度应在3-20个字符之间' }
            ]}
          >
            <Input 
              placeholder="请输入用户名" 
              disabled={isLoading || isLocked}
              prefix={<UserOutlined style={{ color: '#64748b' }} />}
              style={{
                background: 'rgba(15, 23, 42, 0.6)',
                border: '1px solid rgba(148, 163, 184, 0.2)',
                borderRadius: '12px',
                color: '#f1f5f9',
                height: '44px'
              }}
            />
          </Form.Item>
          <Form.Item
            name="password"
            label="密码"
            rules={[
              { required: true, message: '请输入密码' },
              { min: 6, message: '密码长度至少6个字符' }
            ]}
          >
            <Input.Password 
              placeholder="请输入密码" 
              disabled={isLoading || isLocked}
              prefix={<LockOutlined style={{ color: '#64748b' }} />}
              iconRender={(visible) => (
                <EyeOutlined 
                  style={{ color: '#64748b', cursor: 'pointer' }} 
                  onClick={() => setShowPassword(!visible)} 
                />
              )}
              style={{
                background: 'rgba(15, 23, 42, 0.6)',
                border: '1px solid rgba(148, 163, 184, 0.2)',
                borderRadius: '12px',
                color: '#f1f5f9',
                height: '44px'
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
                background: 'linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%)',
                border: 'none',
                boxShadow: '0 4px 14px rgba(59, 130, 246, 0.4)'
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
            borderTop: '1px solid rgba(255, 255, 255, 0.08)',
            textAlign: 'center'
          }}
        >
          <p style={{ color: '#64748b', fontSize: '13px', margin: 0 }}>
            默认账号：<span style={{ color: '#94a3b8' }}>admin</span>（初始密码由部署配置决定，请联系管理员获取）
          </p>
        </div>
      </Card>

      <style>{`
        @keyframes float {
          0%, 100% {
            transform: translate(0, 0);
          }
          25% {
            transform: translate(20px, -20px);
          }
          50% {
            transform: translate(-20px, 20px);
          }
          75% {
            transform: translate(10px, 10px);
          }
        }

        @media screen and (max-width: 768px) {
          .login-card {
            width: calc(100% - 32px) !important;
            margin: 0 16px;
          }
          
          .float-animation {
            animation: none !important;
          }
        }

        @media screen and (max-width: 480px) {
          .login-card {
            padding: 20px !important;
          }
        }
      `}</style>
    </div>
  );
};

export default Login;
