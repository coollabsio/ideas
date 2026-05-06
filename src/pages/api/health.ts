import type { APIRoute } from 'astro';
import { db } from '~/lib/db';

export const prerender = false;

const startedAt = Date.now();

export const GET: APIRoute = () => {
  try {
    db.prepare('SELECT 1').get();
    return Response.json({
      status: 'ok',
      uptimeSec: Math.floor((Date.now() - startedAt) / 1000),
    });
  } catch (err) {
    return new Response(
      JSON.stringify({ status: 'error', error: (err as Error).message }),
      { status: 503, headers: { 'content-type': 'application/json' } }
    );
  }
};
