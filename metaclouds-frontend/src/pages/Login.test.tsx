/**
 * Login 页面组件测试
 *
 * 测试范围：
 * 1. 表单渲染（用户名/密码输入框、登录按钮）
 * 2. 输入验证（空值提交显示 antd Form 校验错误）
 * 3. 登录成功（mock API 返回 token，验证导航到 /dashboard）
 * 4. 登录失败（mock API 返回错误，验证错误消息显示）
 *
 * Mock 策略：
 * - useLoginMutation：用 jest.mock 替换为 mockMutationHook
 * - useNavigate：用 jest.mock 替换为 jest.fn()，断言导航行为
 * - 不依赖外部网络，全部 API 调用均为 mock
 */
import React from 'react';
import { render, screen, waitFor, fireEvent } from '../test/test-utils';
import { mockMutationHook, mockEnvelope } from '../test/mocks';
import Login from './Login';

// ===== Mock 配置 =====

// mock useNavigate，断言登录成功后是否导航到 /dashboard
const mockNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useNavigate: () => mockNavigate,
}));

// mock useLoginMutation，每个测试可通过 mockLoginImplementation 覆盖行为
let mockLoginTrigger = jest.fn();
jest.mock('../store/api', () => ({
  ...jest.requireActual('../store/api'),
  useLoginMutation: () => [
    mockLoginTrigger,
    { isLoading: false, isSuccess: false, isError: false, error: null, data: null, reset: jest.fn() },
  ],
}));

describe('Login 页面', () => {
  // 每个测试前重置 mock
  beforeEach(() => {
    mockNavigate.mockClear();
    mockLoginTrigger = jest.fn();
  });

  // ===== 1. 表单渲染 =====

  describe('表单渲染', () => {
    test('渲染用户名输入框', () => {
      render(<Login />);
      // antd Input 的 placeholder 可作为可访问名称
      expect(screen.getByPlaceholderText('请输入用户名')).toBeInTheDocument();
    });

    test('渲染密码输入框', () => {
      render(<Login />);
      expect(screen.getByPlaceholderText('请输入密码')).toBeInTheDocument();
    });

    test('渲染登录按钮', () => {
      render(<Login />);
      // 登录按钮文字含空格 "登 录"，用正则匹配
      expect(screen.getByRole('button', { name: /登\s*录/ })).toBeInTheDocument();
    });

    test('渲染品牌标题 Metaclouds', () => {
      render(<Login />);
      expect(screen.getByText('Metaclouds')).toBeInTheDocument();
    });

    test('渲染副标题"算力调度平台"', () => {
      render(<Login />);
      expect(screen.getByText('算力调度平台')).toBeInTheDocument();
    });
  });

  // ===== 2. 输入验证 =====

  describe('输入验证', () => {
    test('空值提交显示用户名必填错误', async () => {
      render(<Login />);
      // 直接点击登录按钮（不输入任何内容）
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));
      // antd Form 异步校验，需等待错误消息出现
      await waitFor(() => {
        expect(screen.getByText('请输入用户名')).toBeInTheDocument();
      });
    });

    test('空值提交显示密码必填错误', async () => {
      render(<Login />);
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));
      await waitFor(() => {
        expect(screen.getByText('请输入密码')).toBeInTheDocument();
      });
    });

    test('用户名长度不足 3 字符时显示长度错误', async () => {
      render(<Login />);
      // 输入 2 个字符的用户名
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'ab' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: '123456' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));
      await waitFor(() => {
        expect(screen.getByText('用户名长度应在3-20个字符之间')).toBeInTheDocument();
      });
    });

    test('密码长度不足 6 字符时显示长度错误', async () => {
      render(<Login />);
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'admin' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: '123' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));
      await waitFor(() => {
        expect(screen.getByText('密码长度至少6个字符')).toBeInTheDocument();
      });
    });

    test('校验未通过时不调用登录 API', async () => {
      render(<Login />);
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));
      // 等待校验完成
      await waitFor(() => {
        expect(screen.getByText('请输入用户名')).toBeInTheDocument();
      });
      // 登录 API 不应被调用
      expect(mockLoginTrigger).not.toHaveBeenCalled();
    });
  });

  // ===== 3. 登录成功 =====

  describe('登录成功', () => {
    beforeEach(() => {
      // 配置登录成功：返回包含 user 和 expires_at 的响应
      mockLoginTrigger = jest.fn().mockReturnValue({
        unwrap: jest.fn().mockResolvedValue(
          mockEnvelope({
            token: 'mock-jwt-token',
            user: { id: 1, username: 'admin', email: 'admin@example.com', role: 'admin', tenant_id: 1 },
            expires_at: Math.floor(Date.now() / 1000) + 86400,
          }),
        ),
      });
      // 重新 mock useLoginMutation 使用新的 trigger
      jest.mock('../store/api', () => ({
        ...jest.requireActual('../store/api'),
        useLoginMutation: () => [
          mockLoginTrigger,
          { isLoading: false, isSuccess: true, isError: false, error: null, data: null, reset: jest.fn() },
        ],
      }));
    });

    test('输入有效凭据后调用登录 API', async () => {
      render(<Login />);
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'admin' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: 'password123' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));

      await waitFor(() => {
        expect(mockLoginTrigger).toHaveBeenCalledWith({
          username: 'admin',
          password: 'password123',
        });
      });
    });

    test('登录成功后导航到 /dashboard', async () => {
      render(<Login />);
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'admin' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: 'password123' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));

      await waitFor(() => {
        expect(mockNavigate).toHaveBeenCalledWith('/dashboard');
      });
    });

    test('登录成功后将用户信息写入 localStorage', async () => {
      render(<Login />);
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'admin' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: 'password123' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));

      await waitFor(() => {
        const userStr = localStorage.getItem('user');
        expect(userStr).not.toBeNull();
        const user = JSON.parse(userStr!);
        expect(user.username).toBe('admin');
        expect(user.role).toBe('admin');
      });
    });
  });

  // ===== 4. 登录失败 =====

  describe('登录失败', () => {
    beforeEach(() => {
      // 配置登录失败：unwrap reject 错误
      mockLoginTrigger = jest.fn().mockReturnValue({
        unwrap: jest.fn().mockRejectedValue({
          status: 401,
          data: { message: '用户名或密码错误' },
        }),
      });
    });

    test('登录失败时显示错误消息', async () => {
      render(<Login />);
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'wronguser' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: 'wrongpass' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));

      // antd Alert 显示错误消息
      await waitFor(() => {
        expect(screen.getByText('用户名或密码错误')).toBeInTheDocument();
      });
    });

    test('登录失败时不导航', async () => {
      render(<Login />);
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'wronguser' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: 'wrongpass' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));

      await waitFor(() => {
        expect(screen.getByText('用户名或密码错误')).toBeInTheDocument();
      });
      expect(mockNavigate).not.toHaveBeenCalled();
    });

    test('登录失败时显示"登录失败"Alert 标题', async () => {
      render(<Login />);
      fireEvent.change(screen.getByPlaceholderText('请输入用户名'), { target: { value: 'wronguser' } });
      fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: 'wrongpass' } });
      fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }));

      await waitFor(() => {
        expect(screen.getByText('登录失败')).toBeInTheDocument();
      });
    });
  });
});
