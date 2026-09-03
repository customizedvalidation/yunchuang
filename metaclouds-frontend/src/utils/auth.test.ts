import { readStoredRole, isRoleAllowed, hasPermission, KNOWN_ROLES } from './auth';
import type { Permission } from './auth';

describe('readStoredRole', () => {
  beforeEach(() => localStorage.clear());

  it('returns null when no user stored', () => {
    expect(readStoredRole()).toBeNull();
  });

  it('returns the role parsed from the stored user object', () => {
    localStorage.setItem(
      'user',
      JSON.stringify({ username: 'alice', email: 'alice@example.com', role: 'admin' }),
    );
    expect(readStoredRole()).toBe('admin');
  });

  it('returns null for a role not in KNOWN_ROLES', () => {
    localStorage.setItem('user', JSON.stringify({ username: 'mallory', role: 'superuser' }));
    expect(readStoredRole()).toBeNull();
  });

  it('returns null for corrupt JSON', () => {
    localStorage.setItem('user', '{not valid json');
    expect(readStoredRole()).toBeNull();
  });
});

describe('isRoleAllowed', () => {
  it('is fail-open when the current role is unknown (null)', () => {
    expect(isRoleAllowed(null, ['admin'])).toBe(true);
  });

  it('allows access when the role is in the constraint', () => {
    expect(isRoleAllowed('manager', ['admin', 'manager'])).toBe(true);
  });

  it('denies access when the role is outside the constraint', () => {
    expect(isRoleAllowed('user', ['admin', 'manager'])).toBe(false);
  });
});

describe('hasPermission (front-end button-level visibility)', () => {
  beforeEach(() => localStorage.clear());

  it('is fail-open when the stored role is unknown', () => {
    expect(hasPermission('cluster:write' as Permission)).toBe(true);
  });

  it('grants admin every permission', () => {
    localStorage.setItem('user', JSON.stringify({ role: 'admin' }));
    expect(hasPermission('tenant:write' as Permission)).toBe(true);
    expect(hasPermission('admin' as Permission)).toBe(true);
  });

  it('respects the manager permission matrix', () => {
    localStorage.setItem('user', JSON.stringify({ role: 'manager' }));
    expect(hasPermission('cluster:write' as Permission)).toBe(true);
    expect(hasPermission('admin' as Permission)).toBe(false);
  });

  it('respects the user permission matrix', () => {
    localStorage.setItem('user', JSON.stringify({ role: 'user' }));
    expect(hasPermission('job:read' as Permission)).toBe(true);
    expect(hasPermission('job:write' as Permission)).toBe(false);
  });
});

describe('KNOWN_ROLES', () => {
  it('exposes exactly the three supported roles', () => {
    expect(KNOWN_ROLES).toEqual(['admin', 'manager', 'user']);
  });
});
