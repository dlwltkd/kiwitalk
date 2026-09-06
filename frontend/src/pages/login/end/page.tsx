import { Trans } from '@jellybrick/solid-i18next';

import { Button } from '@/ui-common/button';

import * as styles from './page.css';
import { useNavigate } from '@solidjs/router';

export const LoginEndPage = () => {
  const navigate = useNavigate();

  const onStart = () => {
    navigate('/main');
  };

  return (
    <div class={styles.container}>
      <h1 class={styles.title}>
        <Trans key={'login.end_caption_title'} />
      </h1>
      <span class={styles.subtitle}>
        <Trans key={'login.end_caption_subtitle'} />
      </span>
      <Button onClick={onStart}>
        <Trans key={'login.start'} />
      </Button>
    </div>
  );
};
