<script setup lang="ts">
import ManagementView from "./views/ManagementView.vue";
import OverlayView from "./views/OverlayView.vue";

const view = new URLSearchParams(window.location.search).get("view") ?? "management";
</script>

<template>
  <OverlayView v-if="view === 'overlay'" />
  <ManagementView v-else />
</template>

