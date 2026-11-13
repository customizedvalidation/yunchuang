<template>
  <main id="mc-main" class="login-page">
    <!-- 左侧品牌展示区：抽象算力网格，纯 SVG，无外部资源 -->
    <section class="login-brand" aria-label="Metaclouds 平台介绍">
      <svg class="login-mesh" viewBox="0 0 640 640" fill="none" aria-hidden="true">
        <defs>
          <pattern id="mc-dot-grid" width="32" height="32" patternUnits="userSpaceOnUse">
            <circle cx="1.5" cy="1.5" r="1.1" fill="rgba(255,255,255,0.18)" />
          </pattern>
          <linearGradient id="mc-mesh-line" x1="0" y1="0" x2="640" y2="640" gradientUnits="userSpaceOnUse">
            <stop stop-color="#69b1ff" stop-opacity="0.55" />
            <stop offset="1" stop-color="#2f6bff" stop-opacity="0.05" />
          </linearGradient>
        </defs>
        <g>
          <rect width="640" height="640" fill="url(#mc-dot-grid)" />
          <path d="M-20 430 C 120 370, 210 470, 340 390 S 540 300, 660 350" stroke="url(#mc-mesh-line)" stroke-width="1.5" />
          <path d="M-20 470 C 130 410, 220 510, 350 430 S 550 340, 660 390" stroke="rgba(105,177,255,0.16)" stroke-width="1" />
          <path d="M-20 390 C 110 330, 200 430, 330 350 S 530 260, 660 310" stroke="rgba(105,177,255,0.16)" stroke-width="1" />
          <circle cx="340" cy="390" r="4" fill="#8fc0ff" />
          <circle cx="340" cy="390" r="12" stroke="rgba(143,192,255,0.4)" stroke-width="1" />
          <circle cx="120" cy="370" r="3" fill="rgba(143,192,255,0.7)" />
          <circle cx="540" cy="300" r="3" fill="rgba(143,192,255,0.5)" />
        </g>
      </svg>

      <div class="login-brand-content">
        <div class="login-brand-logo">
          <div class="login-logo-badge">
            <el-icon :size="24" color="#fff"><Promotion /></el-icon>
          </div>
          <div class="login-brand-wordmark">
            <div class="login-brand-name">Metaclouds</div>
            <div class="login-brand-sub">企业级算力调度平台</div>
          </div>
        </div>

        <div class="login-brand-eyebrow">Enterprise Compute Platform</div>
        <h1 class="login-brand-title">
          智能算力调度<br />
          <span class="login-brand-title-accent">驱动 AI 创新</span>
        </h1>
        <p class="login-brand-desc">
          统一管理多集群 GPU 资源，提供从训练到推理的全链路算力服务，
          让每一次计算都高效、稳定、安全。
        </p>

        <ul class="login-features">
          <li v-for="(f, i) in FEATURES" :key="f.title" class="login-feature">
            <span class="login-feature-index">0{{ i + 1 }}</span>
            <span class="login-feature-copy">
              <span class="login-feature-title">{{ f.title }}</span>
              <span class="login-feature-desc">{{ f.desc }}</span>
            </span>
          </li>
        </ul>
      </div>

      <div class="login-brand-footer">
        <span>© 2026 Metaclouds. All rights reserved.</span>
        <span>Version 2.0.1</span>
      </div>
    </section>

    <!-- 右侧登录表单区 -->
    <section class="login-form-section">
      <!-- 未登录用户也可切换外观；与顶栏共用同一个三态状态机 -->
      <div class="login-theme-switch"><ThemeToggle /></div>
      <div class="login-mobile-logo">
        <div class="login-logo-badge is-mobile">
          <el-icon :size="22" color="#fff"><Promotion /></el-icon>
        </div>
        <div class="login-mobile-name">Metaclouds</div>
      </div>

      <div class="login-card">
        <div class="login-card-head">
          <h2>欢迎回来</h2>
          <p>登录您的账户以继续使用算力调度平台</p>
        </div>

        <el-alert
          v-if="isLocked"
          title="账号已锁定"
          description="连续失败次数过多，请稍后再试。"
          type="error"
          :closable="false"
          show-icon
          class="login-alert"
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
              <template #prefix><el-icon color="var(--mc-text-3)"><User /></el-icon></template>
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
              <template #prefix><el-icon color="var(--mc-text-3)"><Lock /></el-icon></template>
            </el-input>
          </el-form-item>

          <!-- 行内错误：与 toast 并存，读屏可感知且不依赖瞬时消息 -->
          <div v-if="formError" class="login-form-error" role="alert">{{ formError }}</div>

          <el-button
            type="primary"
            size="large"
            class="login-submit"
            :loading="loading"
            :disabled="isLocked"
            @click="onSubmit"
          >
            {{ loading ? '登录中...' : '登录' }}
          </el-button>
        </el-form>

        <div class="login-hint">
          <span>登录遇到问题？请联系系统管理员</span>
          <span class="login-hint-safety">请勿共享您的账户凭据</span>
        </div>
      </div>

      <div class="login-mobile-footer">© 2026 Metaclouds. All rights reserved.</div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { onUnmounted, reactive, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus/es/components/message/index'
import type { FormInstance, FormRules } from 'element-plus'
import { User, Lock, Promotion } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import ThemeToggle from '@/components/ThemeToggle.vue'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const FEATURES = [
  {
    title: '多集群统一调度',
    desc: '跨集群资源池化管理，支持 K8s / Slurm / LSF 混合调度',
  },
  {
    title: 'GPU 细粒度分配',
    desc: '支持 1/2、1/4 GPU 切分与显存超发，资源利用率提升 30%+',
  },
  {
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
const formError = ref('')
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
    formError.value = '登录失败次数过多，请稍后再试'
    ElMessage.error(formError.value)
    return
  }
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }

  loading.value = true
  formError.value = ''
  try {
    // 走 auth store：成功后会同步更新 pinia user（isLoggedIn=true）并持久化，
    // 否则路由守卫仍会把已登录用户弹回 /login。
    await authStore.login(form.username, form.password)
    ElMessage.success('登录成功')
    loginAttempts.value = 0
    const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/dashboard'
    router.push(redirect || '/dashboard')
  } catch (e) {
    loginAttempts.value += 1
    if (loginAttempts.value >= 5) {
      isLocked.value = true
      formError.value = '登录失败次数过多，账号已被锁定 1 分钟'
      ElMessage.error(formError.value)
      lockTimer = setTimeout(() => {
        isLocked.value = false
        loginAttempts.value = 0
        formError.value = ''
      }, 60_000)
    } else {
      formError.value = errMessage(e)
      ElMessage.error(formError.value)
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
  background: var(--mc-bg);
}

/* ============ 左侧品牌区 ============ */
.login-brand {
  position: relative;
  flex: 0 0 50%;
  background: #0b1424;
  display: flex;
  flex-direction: column;
  padding: var(--mc-space-12) clamp(32px, 5vw, 72px);
  overflow: hidden;
}
.login-mesh {
  position: absolute;
  right: -6%;
  bottom: -10%;
  width: min(700px, 88%);
  height: auto;
  pointer-events: none;
  opacity: 0.72;
  /* 右下角最实、向左上柔和隐去：给深色区一层结构感，又不抢文字 */
  -webkit-mask-image: linear-gradient(to top left, #000 0%, rgba(0, 0, 0, 0.55) 45%, transparent 88%);
  mask-image: linear-gradient(to top left, #000 0%, rgba(0, 0, 0, 0.55) 45%, transparent 88%);
}
.login-brand-content {
  position: relative;
  z-index: 1;
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  max-width: 520px;
}
.login-brand-logo { display: flex; align-items: center; gap: var(--mc-space-3); margin-bottom: clamp(48px, 8vh, 88px); }
.login-logo-badge {
  width: 40px; height: 40px;
  border-radius: var(--mc-radius-md);
  background: var(--mc-brand-grad);
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
}
.login-logo-badge.is-mobile { width: 44px; height: 44px; margin: 0 auto var(--mc-space-3); }
.login-brand-name { font-size: 17px; font-weight: var(--mc-fw-semibold); color: #fff; letter-spacing: -0.01em; }
.login-brand-sub { font-size: 11.5px; color: rgba(255, 255, 255, 0.5); margin-top: 2px; }

.login-brand-eyebrow {
  font-size: 11px;
  font-weight: var(--mc-fw-semibold);
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--sidebar-accent-color);
  margin-bottom: var(--mc-space-4);
}
.login-brand-title {
  margin: 0 0 var(--mc-space-5);
  font-size: clamp(34px, 3.2vw, 46px);
  line-height: 1.24;
  font-weight: var(--mc-fw-semibold);
  letter-spacing: -0.025em;
  color: #fff;
}
.login-brand-title-accent { color: #9cc6ff; }
.login-brand-desc {
  margin: 0 0 clamp(32px, 5vh, 56px);
  font-size: var(--mc-fs-md);
  line-height: var(--mc-lh-relaxed);
  color: rgba(255, 255, 255, 0.62);
  max-width: 440px;
}

.login-features { list-style: none; margin: 0; padding: 0; max-width: 460px; }
.login-feature {
  display: flex;
  gap: var(--mc-space-4);
  padding: var(--mc-space-3) 0;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}
.login-feature:last-child { border-bottom: 1px solid rgba(255, 255, 255, 0.1); }
.login-feature-index {
  flex: none;
  font-family: var(--mc-font-mono);
  font-size: 11px;
  color: rgba(143, 192, 255, 0.7);
  padding-top: 3px;
  letter-spacing: 0.04em;
}
.login-feature-copy { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.login-feature-title { font-size: 14px; font-weight: var(--mc-fw-medium); color: #fff; }
.login-feature-desc { font-size: 12.5px; color: rgba(255, 255, 255, 0.5); line-height: var(--mc-lh-snug); }

.login-brand-footer {
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: space-between;
  gap: var(--mc-space-4);
  font-size: 11.5px;
  color: rgba(255, 255, 255, 0.38);
}

/* ============ 右侧表单区 ============ */
.login-form-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--mc-space-10) var(--mc-space-6);
  position: relative;
}
.login-theme-switch { position: absolute; top: var(--mc-space-5); right: var(--mc-space-6); z-index: 2; }
.login-mobile-logo { display: none; text-align: center; margin-bottom: var(--mc-space-6); }
.login-mobile-name { font-size: 18px; font-weight: var(--mc-fw-semibold); color: var(--mc-text-1); }

.login-card {
  width: 100%;
  max-width: 400px;
  background: var(--mc-surface);
  border: 1px solid var(--mc-line);
  border-radius: var(--mc-radius-lg);
  box-shadow: var(--mc-shadow-3);
  padding: var(--mc-space-10) var(--mc-space-8);
}
.login-card-head { margin-bottom: var(--mc-space-8); }
.login-card-head h2 {
  margin: 0 0 var(--mc-space-2);
  font-size: var(--mc-fs-2xl);
  font-weight: var(--mc-fw-semibold);
  letter-spacing: -0.02em;
  color: var(--mc-text-1);
}
.login-card-head p { margin: 0; font-size: var(--mc-fs-sm); line-height: var(--mc-lh-snug); color: var(--mc-text-3); }

.login-alert { margin-bottom: var(--mc-space-5); }
.login-form-error {
  margin-bottom: var(--mc-space-4);
  padding: var(--mc-space-2) var(--mc-space-3);
  border: 1px solid rgba(194, 24, 91, 0.2);
  border-radius: var(--mc-radius-sm);
  background: var(--mc-danger-soft);
  color: var(--mc-danger-fg);
  font-size: var(--mc-fs-sm);
  line-height: var(--mc-lh-snug);
}
.login-submit {
  width: 100%;
  height: 46px;
  margin-top: var(--mc-space-2);
  font-size: var(--mc-fs-body);
  font-weight: var(--mc-fw-semibold);
}
.login-hint {
  margin-top: var(--mc-space-8);
  padding-top: var(--mc-space-5);
  border-top: 1px solid var(--mc-line);
  display: flex;
  flex-direction: column;
  gap: var(--mc-space-1);
  text-align: center;
  color: var(--mc-text-3);
  font-size: var(--mc-fs-sm);
  line-height: var(--mc-lh-snug);
}
.login-hint-safety { font-size: var(--mc-fs-xs); color: var(--mc-text-3); opacity: 0.8; }
.login-mobile-footer { display: none; margin-top: var(--mc-space-6); font-size: var(--mc-fs-xs); color: var(--mc-text-3); }

/* ============ 响应式 ============ */
@media (max-width: 1100px) {
  .login-brand { flex-basis: 46%; padding-inline: var(--mc-space-8); }
}
@media (max-width: 900px) {
  .login-brand { display: none; }
  .login-mobile-logo { display: block; }
  .login-mobile-footer { display: block; }
  .login-form-section { padding: var(--mc-space-8) var(--mc-space-4); }
}
@media (max-width: 480px) {
  .login-card { padding: var(--mc-space-8) var(--mc-space-5); border-radius: var(--mc-radius-md); }
}
</style>
