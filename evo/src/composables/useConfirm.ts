import { ref } from 'vue';

interface ConfirmOptions {
  title?: string;
  message: string;
}

interface ConfirmState {
  show: boolean;
  title: string;
  message: string;
  resolve: (value: boolean) => void;
}

const state = ref<ConfirmState>({
  show: false,
  title: '',
  message: '',
  resolve: () => {},
});

export function useConfirm() {
  const confirm = (messageOrOptions: string | ConfirmOptions): Promise<boolean> => {
    return new Promise((resolve) => {
      state.value.resolve = resolve;
      
      if (typeof messageOrOptions === 'string') {
        state.value.message = messageOrOptions;
        state.value.title = '';
      } else {
        state.value.message = messageOrOptions.message;
        state.value.title = messageOrOptions.title || '';
      }
      
      state.value.show = true;
    });
  };

  const handleConfirm = () => {
    state.value.show = false;
    state.value.resolve(true);
  };

  const handleCancel = () => {
    state.value.show = false;
    state.value.resolve(false);
  };

  return {
    state,
    confirm,
    handleConfirm,
    handleCancel,
  };
}
