<template>
  <main class="login-page">
    <!-- 左侧品牌展示区 -->
    <section class="login-brand">
      <div class="login-grid-bg" />
      <div class="login-glow login-glow-1" />
      <div class="login-glow login-glow-2" />

      <div class="login-brand-content">
        <div class="login-brand-logo">
          <div class="login-logo-badge">
            <el-icon :size="28" color="#fff"><Promotion /></el-icon>
          </div>
          <div>
            <div class="login-brand-name">Metaclouds</div>
            <div class="login-brand-sub">企业级算力调度平台</div>
          </div>
        </div>

        <h1 class="login-brand-title">
          智能算力调度
          <br />
          <span class="grad-text">驱动 AI 创新</span>
        </h1>
        <p class="login-brand-desc">
          统一管理多集群 GPU 资源，提供从训练到推理的全链路算力服务，
          让每一次计算都高效、稳定、安全。
        </p>

        <div class="login-features">
          <div v-for="f in FEATURES" :key="f.title" class="login-feature-item">
            <div class="login-feature-icon">
              <el-icon :size="20" color="#fff"><component :is="f.icon" /></el-icon>
            </div>
            <div>
              <div class="login-feature-title">{{ f.title }}</div>
              <div class="login-feature-desc">{{ f.desc }}</div>
            </div>
          </div>
        </div>
      </div>

      <div class="login-brand-footer">
        <span>© 2026 Metaclouds. All rights reserved.</span>
        <span>Version 2.0.0</span>
      </div>
    </section>

    <!-- 右侧登录表单区 -->
    <section class="login-form-section">
      <div class="login-mobile-logo">
        <div class="login-logo-badge is-mobile">
          <el-icon :size="24" color="#fff"><Promotion /></el-icon>
        </div>
        <div class="login-mobile-name">Metaclouds</div>
      </div>

      <div class="login-card">
        <div class="login-card-strip" />
        <div class="login-card-head">
          <h2>欢迎回来</h2>
          <p>登录您的账户以继续使用算力调度平台</p>
        </div>

        <el-alert
          v-if="isLocked"
          title="账号已锁定"
          type="error"
          :closable="false"
          show-icon
          style="margin-bottom: 16px"
        />

        <el-form
          ref="formRef"
          :model="form"
          :rules="rules"
          label-position="top"
          @keyup.enter="onSubmit"
        >
          <el-form-item label="用户名" prop="username">
            <el-input
              v-model="form.username"
              placeholder="请输入用户名"
              :disabled="loading || isLocked"
              autocomplete="username"
              size="large"
            >
              <template #prefix><el-icon color="var(--mc-brand)"><User /></el-icon></template>
            </el-input>
          </el-form-item>

          <el-form-item label="密码" prop="password">
            <el-input
              v-model="form.password"
              type="password"
              show-password
              placeholder="请输入密码"
              :disabled="loading || isLocked"
              autocomplete="current-password"
              size="large"
            >
              <template #prefix><el-icon color="var(--mc-brand)"><Lock /></el-icon></template>
            </el-input>
          </el-form-item>

          <el-button
            type="primary"
            size="large"
            class="login-submit"
            :loading="loading"
            :disabled="isLocked"
            @click="onSubmit"
          >
            {{ loading ? '登录中...' : '登 录' }}
          </el-button>
        </el-form>

        <div class="login-hint">
          默认账号：<span class="login-hint-user">admin</span>
          <br />
          初始密码由部署配置决定，请联系管理员获取
        </div>
      </div>

      <div class="login-mobile-footer">© 2026 Metaclouds. All rights reserved.</div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { onUnmounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { User, Lock, Promotion, Connection, Lightning } from '@element-plus/icons-vue'
import { authApi } from '@/api'

const router = useRouter()

const FEATURES = [
  {
    icon: Connection,
    title: '多集群统一调度',
    desc: '跨集群资源池化管理，支持 K8s / Slurm / LSF 混合调度',
  },
  {
    icon: Lightning,
    title: 'GPU 细粒度分配',
    desc: '支持 1/2、1/4 GPU 切分与显存超发，资源利用率提升 30%+',
  },
  {
    icon: Lock,
    title: '企业级安全防护',
    desc: 'RBAC 权限体系 + 多租户隔离 + 审计日志，合规无忧',
  },
]

const formRef = ref<FormInstance>()
const form = reactive({ username: '', password: '' })
const rules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '用户名长度应在3-20个字符之间', trigger: 'blur' },
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6个字符', trigger: 'blur' },
  ],
}

const loading = ref(false)
const loginAttempts = ref(0)
const isLocked = ref(false)
let lockTimer: ReturnType<typeof setTimeout> | undefined

/** 从 axios 错误中提取可读信息 */
function errMessage(e: unknown): string {
  const resp = (e as { response?: { data?: { message?: string } } })?.response
  return resp?.data?.message || '请检查用户名和密码'
}

async function onSubmit() {
  if (isLocked.value) {
    ElMessage.error('登录失败次数过多，请稍后再试')
    return
  }
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }

  loading.value = true
  try {
    const { user, expires_at } = await authApi.login({
      username: form.username,
      password: form.password,
    })
    localStorage.setItem(
      'user',
      JSON.stringify({
        username: user?.username ?? '',
        email: user?.email ?? '',
        role: user?.role ?? '',
      }),
    )
    localStorage.setItem('auth_expiry', String((expires_at ?? 0) * 1000))
    ElMessage.success('登录成功')
    loginAttempts.value = 0
    router.push('/dashboard')
  } catch (e) {
    loginAttempts.value += 1
    if (loginAttempts.value >= 5) {
      isLocked.value = true
      ElMessage.error('登录失败次数过多，账号已被锁定1分钟')
      lockTimer = setTimeout(() => {
        isLocked.value = false
        loginAttempts.value = 0
      }, 60_000)
    } else {
      ElMessage.error(errMessage(e))
    }
  } finally {
    loading.value = false
  }
}

onUnmounted(() => {
  if (lockTimer) clearTimeout(lockTimer)
})
</script>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  position: relative;
  overflow: hidden;
  background: linear-gradient(135deg, #f0f5ff 0%, #f8faff 50%, #eef4ff 100%);
}

/* ============ 左侧品牌区 ============ */
.login-brand {
  position: relative;
  flex: 0 0 52%;
  background: linear-gradient(135deg, #0b1f4d 0%, #123a8f 55%, #0b1f4d 100%);
  display: flex;
  flex-direction: column;
  padding: 48px 56px;
  overflow: hidden;
}
.login-grid-bg {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(255, 255, 255, 0.05) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.05) 1px, transparent 1px);
  background-size: 44px 44px;
  mask-image: radial-gradient(ellipse at center, #000 40%, transparent 80%);
  pointer-events: none;
}
.login-glow {
  position: absolute;
  border-radius: 50%;
  pointer-events: none;
}
.login-glow-1 {
  top: -10%;
  right: -20%;
  width: 500px;
  height: 500px;
  background: radial-gradient(circle, rgba(47, 107, 255, 0.45) 0%, transparent 60%);
}
.login-glow-2 {
  bottom: -15%;
  left: -10%;
  width: 420px;
  height: 420px;
  background: radial-gradient(circle, rgba(105, 177, 255, 0.3) 0%, transparent 60%);
}

.login-brand-content {
  position: relative;
  z-index: 1;
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  max-width: 560px;
}
.login-brand-logo {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 56px;
}
.login-logo-badge {
  width: 56px;
  height: 56px;
  border-radius: 16px;
  background: linear-gradient(135deg, #2f6bff 0%, #5b3fd9 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 40px rgba(47, 107, 255, 0.5);
  flex-shrink: 0;
}
.login-logo-badge.is-mobile { width: 48px; height: 48px; border-radius: 14px; margin: 0 auto 12px; }
.login-brand-name { font-size: 22px; font-weight: 700; color: #fff; letter-spacing: 0.5px; }
.login-brand-sub { font-size: 13px; color: rgba(255, 255, 255, 0.6); margin-top: 2px; }

.login-brand-title {
  margin: 0 0 18px;
  font-size: 44px;
  line-height: 1.2;
  font-weight: 700;
  color: #fff;
  letter-spacing: 1px;
}
.grad-text {
  background: linear-gradient(135deg, #69b1ff 0%, #5b8bff 100%);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}
.login-brand-desc {
  margin: 0 0 40px;
  font-size: 15px;
  line-height: 1.8;
  color: rgba(255, 255, 255, 0.65);
  max-width: 460px;
}

.login-features { display: flex; flex-direction: column; gap: 22px; }
.login-feature-item { display: flex; align-items: flex-start; gap: 14px; }
.login-feature-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.login-feature-title { font-size: 15px; font-weight: 600; color: #fff; margin-bottom: 3px; }
.login-feature-desc { font-size: 13px; color: rgba(255, 255, 255, 0.55); line-height: 1.6; }

.login-brand-footer {
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
}

/* ============ 右侧表单区 ============ */
.login-form-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 24px;
  position: relative;
}
.login-mobile-logo { display: none; text-align: center; margin-bottom: 24px; }
.login-mobile-name { font-size: 20px; font-weight: 700; color: var(--mc-text-1); }

.login-card {
  width: 100%;
  max-width: 420px;
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border: 1px solid rgba(47, 107, 255, 0.18);
  border-radius: 24px;
  box-shadow: 0 25px 60px rgba(11, 31, 77, 0.18), 0 0 40px rgba(47, 107, 255, 0.15);
  padding: 40px 36px;
  position: relative;
  overflow: hidden;
}
.login-card-strip {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(135deg, #2f6bff 0%, #5b3fd9 100%);
}
.login-card-head { text-align: center; margin-bottom: 28px; }
.login-card-head h2 { margin: 0 0 8px; font-size: 26px; font-weight: 700; color: var(--mc-text-1); }
.login-card-head p { margin: 0; font-size: 14px; color: var(--mc-text-3); }

.login-submit {
  width: 100%;
  height: 50px;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 2px;
  border: none;
  box-shadow: 0 6px 20px rgba(47, 107, 255, 0.4);
}

.login-hint {
  margin-top: 28px;
  padding-top: 20px;
  border-top: 1px solid var(--mc-line);
  text-align: center;
  color: var(--mc-text-3);
  font-size: 13px;
  line-height: 1.7;
}
.login-hint-user { color: var(--mc-brand); font-weight: 600; }

.login-mobile-footer { display: none; margin-top: 20px; font-size: 12px; color: var(--mc-text-3); }

/* ============ 响应式 ============ */
@media (max-width: 900px) {
  .login-brand { display: none; }
  .login-mobile-logo { display: block; }
  .login-mobile-footer { display: block; }
}
</style>
