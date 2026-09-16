<template>
  <slot v-if="allowed" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAuthStore } from '@/stores/auth'
import type { UserRole } from '@/types'

const props = defineProps<{
  roles?: UserRole[]
}>()

const auth = useAuthStore()
const allowed = computed(() => {
  if (!props.roles || props.roles.length === 0) return true
  const role = auth.role
  if (!role) return false
  return props.roles.includes(role)
})
</script>
