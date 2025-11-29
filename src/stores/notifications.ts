import { defineStore } from 'pinia';
import { ref } from 'vue';

export type NotificationType = 'success' | 'error' | 'warning' | 'info';

interface NotificationRequest {
  type?: NotificationType;
  title?: string;
  message: string;
  duration?: number;
}

export const useNotificationStore = defineStore('notifications', () => {
  const show = ref(false);
  const type = ref<NotificationType>('info');
  const title = ref<string | undefined>(undefined);
  const message = ref('');
  const duration = ref(1000);

  function notify(req: NotificationRequest) {
    // If already showing, force reset (simple implementation, or queue)
    show.value = false;
    
    setTimeout(() => {
      type.value = req.type || 'info';
      title.value = req.title;
      message.value = req.message;
      duration.value = req.duration ?? 1000;
      show.value = true;
    }, 50);
  }

  function success(msg: string, titleStr?: string, durationMs?: number) {
    notify({ type: 'success', message: msg, title: titleStr, duration: durationMs });
  }

  function error(msg: string, titleStr?: string, durationMs?: number) {
    notify({ type: 'error', message: msg, title: titleStr, duration: durationMs });
  }

  function warning(msg: string, titleStr?: string, durationMs?: number) {
    notify({ type: 'warning', message: msg, title: titleStr, duration: durationMs });
  }

  function info(msg: string, titleStr?: string, durationMs?: number) {
    notify({ type: 'info', message: msg, title: titleStr, duration: durationMs });
  }

  function close() {
    show.value = false;
  }

  return {
    show,
    type,
    title,
    message,
    duration,
    notify,
    success,
    error,
    warning,
    info,
    close
  };
});
