import adapter from '@sveltejs/adapter-static';

export default {
  kit: {
    adapter: adapter({ fallback: '200.html' })
  },
  compilerOptions: {
    runes: false
  }
};
