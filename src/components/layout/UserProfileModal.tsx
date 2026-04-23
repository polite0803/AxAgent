import { useState } from 'react';
import { Modal, Input, Avatar, theme } from 'antd';
import { User } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useUserProfileStore, type AvatarType } from '@/stores';
import { IconEditor } from '@/components/shared/IconEditor';

interface UserProfileModalProps {
  open: boolean;
  onClose: () => void;
}

export function UserProfileModal({ open, onClose }: UserProfileModalProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const profile = useUserProfileStore((s) => s.profile);
  const updateProfile = useUserProfileStore((s) => s.updateProfile);

  const [name, setName] = useState(profile.name);
  const [avatarType, setAvatarType] = useState<AvatarType>(profile.avatarType);
  const [avatarValue, setAvatarValue] = useState(profile.avatarValue);

  const handleSave = () => {
    updateProfile({ name: name.trim(), avatarType, avatarValue });
    onClose();
  };

  return (
    <Modal
      open={open}
      onCancel={onClose}
      mask={{ enabled: true, blur: true }}
      onOk={handleSave}
      okText={t('common.ok')}
      cancelText={t('common.cancel')}
      title={t('userProfile.title')}
      width={400}
      destroyOnHidden
    >
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 16, padding: '16px 0' }}>
        <IconEditor
          iconType={avatarType}
          iconValue={avatarValue}
          onChange={(type, value) => {
            setAvatarType((type as AvatarType) ?? 'icon');
            setAvatarValue(value ?? '');
          }}
          size={72}
          defaultIcon={
            <Avatar
              size={72}
              icon={<User size={16} />}
              style={{ cursor: 'pointer', backgroundColor: token.colorPrimary }}
            />
          }
          showClear={false}
        />

        {/* Name input */}
        <Input
          placeholder={t('userProfile.namePlaceholder')}
          value={name}
          onChange={(e) => setName(e.target.value)}
          style={{ maxWidth: 280 }}
        />
      </div>
    </Modal>
  );
}
