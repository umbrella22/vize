<script setup lang="ts">
// no script bindings needed — this fixture tests URL rewriting only
const dynamicSrc = 'dynamic.png'
</script>

<template>
  <div>
    <!-- relative src: should become an import binding -->
    <img src="./logo.png" alt="logo" />
    <!-- tilde-prefixed module path -->
    <img src="~some-lib/img/icon.svg" alt="icon" />
    <!-- dynamic binding: must NOT be collected -->
    <img :src="dynamicSrc" alt="dynamic" />
    <!-- external: must NOT be collected -->
    <img src="https://cdn.example.com/external.png" alt="external" />
    <!-- video with src + poster -->
    <video src="./intro.mp4" poster="./poster.jpg" controls />
  </div>
</template>
