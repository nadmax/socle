import { Message } from 'discord.js';
import { sleep } from './sleep.js';

async function deleteMessageWithRetry(msg: Message, delayMs: number) {
    try {
        await msg.delete();
    } catch (error: any) {
        if (error.status === 429) {
            const retryAfter = error.retry_after ?? delayMs;
            console.log(`⚠️ Rate limited. Waiting ${retryAfter}ms before retrying message ${msg.id}`);
            await sleep(retryAfter);
            try {
                await msg.delete();
            } catch (err) {
                console.error(`❌ Failed retry deleting message ${msg.id}:`, err);
            }
        } else {
            console.error(`❌ Failed to delete message ${msg.id}:`, error);
        }
    }
}

export async function deleteMessagesConcurrently(
    messages: Iterable<Message>,
    concurrency = 3,
    delayMs = 300
) {
    const queue = Array.from(messages);
    let active = 0;
    let index = 0;

    return new Promise<void>((resolve) => {
        function next() {
            if (index >= queue.length && active === 0) {
                resolve();
                return;
            }

            while (active < concurrency && index < queue.length) {
                const msg = queue[index++];
                active++;
                deleteMessageWithRetry(msg, delayMs)
                    .finally(() => {
                        active--;
                        setTimeout(next, delayMs);
                    });
            }
        }
        next();
    });
}
