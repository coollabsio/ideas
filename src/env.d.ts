/// <reference path="../.astro/types.d.ts" />
/// <reference types="astro/client" />

declare namespace NodeJS {
  interface ProcessEnv {
    GITHUB_CLIENT_ID?: string;
    GITHUB_CLIENT_SECRET?: string;
    GITHUB_LOGIN_ENABLED?: string;
    GITHUB_TOKEN?: string;
    PUBLIC_BASE_URL?: string;
    DB_PATH?: string;
    PORT?: string;
    HOST?: string;
  }
}
