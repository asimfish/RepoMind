import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      component: () => import('@/layouts/MainLayout.vue'),
      children: [
        {
          path: '',
          redirect: '/repos',
        },
        {
          path: 'repos',
          name: 'repos',
          component: () => import('@/views/ReposView.vue'),
        },
        {
          path: 'repos/:id',
          name: 'repo-detail',
          component: () => import('@/views/RepoDetailView.vue'),
        },
        {
          path: 'search',
          name: 'search',
          component: () => import('@/views/SearchView.vue'),
        },
        {
          path: 'settings',
          name: 'settings',
          component: () => import('@/views/SettingsView.vue'),
        },
      ],
    },
    {
      path: '/login',
      name: 'login',
      component: () => import('@/views/LoginView.vue'),
    },
    {
      path: '/oauth-callback',
      name: 'oauth-callback',
      component: () => import('@/views/OAuthCallbackView.vue'),
    },
  ],
})

export default router
