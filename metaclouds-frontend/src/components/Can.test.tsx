/**
 * Can 组件测试
 *
 * 测试范围：
 * 1. 有权限时渲染子元素
 * 2. 无权限时不渲染子元素
 * 3. admin 角色拥有全部权限
 * 4. 未登录（无 user）时 fail-open 放行
 *
 * Can 组件基于 utils/auth 的 hasPermission 函数，
 * 通过 localStorage 中的 user.role 判断当前用户角色。
 */
import React from 'react';
import { render, screen } from '../../test/test-utils';
import { Can } from './Can';

// 辅助函数：设置当前登录用户角色
function setUserRole(role: string | null) {
  if (role === null) {
    localStorage.removeItem('user');
  } else {
    localStorage.setItem('user', JSON.stringify({ username: 'test', role }));
  }
}

describe('Can 组件', () => {
  afterEach(() => {
    localStorage.clear();
  });

  // ===== 1. admin 角色 =====

  describe('admin 角色', () => {
    beforeEach(() => {
      setUserRole('admin');
    });

    test('admin 拥有全部权限，渲染子元素', () => {
      render(
        <Can perm="cluster:write">
          <button>创建集群</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '创建集群' })).toBeInTheDocument();
    });

    test('admin 渲染 tenant:write 权限元素', () => {
      render(
        <Can perm="tenant:write">
          <button>创建租户</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '创建租户' })).toBeInTheDocument();
    });

    test('admin 渲染 security:write 权限元素', () => {
      render(
        <Can perm="security:write">
          <button>编辑策略</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '编辑策略' })).toBeInTheDocument();
    });
  });

  // ===== 2. manager 角色 =====

  describe('manager 角色', () => {
    beforeEach(() => {
      setUserRole('manager');
    });

    test('manager 有 cluster:write 权限，渲染子元素', () => {
      render(
        <Can perm="cluster:write">
          <button>创建集群</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '创建集群' })).toBeInTheDocument();
    });

    test('manager 有 job:submit 权限，渲染子元素', () => {
      render(
        <Can perm="job:submit">
          <button>提交作业</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '提交作业' })).toBeInTheDocument();
    });

    test('manager 有 tenant:read 权限，渲染子元素', () => {
      render(
        <Can perm="tenant:read">
          <span>查看租户</span>
        </Can>,
      );
      expect(screen.getByText('查看租户')).toBeInTheDocument();
    });

    test('manager 无 security:write 权限，不渲染子元素', () => {
      render(
        <Can perm="security:write">
          <button>编辑策略</button>
        </Can>,
      );
      expect(screen.queryByRole('button', { name: '编辑策略' })).not.toBeInTheDocument();
    });
  });

  // ===== 3. user 角色 =====

  describe('user 角色', () => {
    beforeEach(() => {
      setUserRole('user');
    });

    test('user 有 cluster:read 权限，渲染子元素', () => {
      render(
        <Can perm="cluster:read">
          <span>查看集群</span>
        </Can>,
      );
      expect(screen.getByText('查看集群')).toBeInTheDocument();
    });

    test('user 有 job:read 权限，渲染子元素', () => {
      render(
        <Can perm="job:read">
          <span>查看作业</span>
        </Can>,
      );
      expect(screen.getByText('查看作业')).toBeInTheDocument();
    });

    test('user 无 cluster:write 权限，不渲染子元素', () => {
      render(
        <Can perm="cluster:write">
          <button>创建集群</button>
        </Can>,
      );
      expect(screen.queryByRole('button', { name: '创建集群' })).not.toBeInTheDocument();
    });

    test('user 无 job:write 权限，不渲染子元素', () => {
      render(
        <Can perm="job:write">
          <button>取消作业</button>
        </Can>,
      );
      expect(screen.queryByRole('button', { name: '取消作业' })).not.toBeInTheDocument();
    });

    test('user 无 tenant:read 权限，不渲染子元素', () => {
      render(
        <Can perm="tenant:read">
          <span>查看租户</span>
        </Can>,
      );
      expect(screen.queryByText('查看租户')).not.toBeInTheDocument();
    });
  });

  // ===== 4. 未登录 / fail-open =====

  describe('未登录（fail-open 策略）', () => {
    beforeEach(() => {
      setUserRole(null);
    });

    test('未登录时 fail-open 放行，渲染子元素', () => {
      render(
        <Can perm="cluster:write">
          <button>创建集群</button>
        </Can>,
      );
      // 前端 fail-open：读不到角色时放行，真正的授权在后端
      expect(screen.getByRole('button', { name: '创建集群' })).toBeInTheDocument();
    });

    test('user JSON 损坏时 fail-open 放行', () => {
      localStorage.setItem('user', 'not-valid-json');
      render(
        <Can perm="security:write">
          <button>编辑策略</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '编辑策略' })).toBeInTheDocument();
    });

    test('user 中 role 字段缺失时 fail-open 放行', () => {
      localStorage.setItem('user', JSON.stringify({ username: 'test' }));
      render(
        <Can perm="tenant:write">
          <button>创建租户</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '创建租户' })).toBeInTheDocument();
    });

    test('user 中 role 为未知值时 fail-open 放行', () => {
      localStorage.setItem('user', JSON.stringify({ username: 'test', role: 'superadmin' }));
      render(
        <Can perm="cluster:write">
          <button>创建集群</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '创建集群' })).toBeInTheDocument();
    });
  });

  // ===== 5. 渲染多个子元素 =====

  describe('渲染多个子元素', () => {
    test('有权限时渲染多个子元素', () => {
      setUserRole('admin');
      render(
        <Can perm="job:write">
          <button>新建作业</button>
          <button>取消作业</button>
        </Can>,
      );
      expect(screen.getByRole('button', { name: '新建作业' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: '取消作业' })).toBeInTheDocument();
    });

    test('无权限时不渲染任何子元素', () => {
      setUserRole('user');
      render(
        <Can perm="job:write">
          <button>新建作业</button>
          <button>取消作业</button>
        </Can>,
      );
      expect(screen.queryByRole('button', { name: '新建作业' })).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: '取消作业' })).not.toBeInTheDocument();
    });
  });
});
