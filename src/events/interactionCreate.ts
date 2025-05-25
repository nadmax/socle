import {
  Events,
  type Interaction,
  GuildMember,
} from 'discord.js';
import { Event } from '../types/Event.js';

export const event: Event = {
    name: Events.InteractionCreate,
    once: false,

    async execute(interaction: Interaction) {
        if (!interaction.isButton()) {
            return;
        }

        const customId = interaction.customId;
        if (!customId.startsWith('welcome-role-')) {
            return;
        }

        const memberId = customId.split('welcome-role-')[1];
        if (interaction.user.id !== memberId) {
            return interaction.reply({
                content: `❌ Ce bouton n'est pas pour toi!`,
                ephemeral: true,
            });
        }

        const member = interaction.member as GuildMember;
        const roleId = process.env.MEMBER_ROLE_ID!;
        const role = interaction.guild?.roles.cache.get(roleId);

        if (!role) {
            return interaction.reply({
                content: '⚠️ Le rôle "Membre" est introuvable sur le serveur.',
                ephemeral: true,
            });
        }

        if (member.roles.cache.has(role.id)) {
            return interaction.reply({
                content: '✅ Tu as déjà le rôle Membre!',
                ephemeral: true,
            });
        }

        await member.roles.add(role);
        await interaction.reply({
            content: `🎉 Bienvenue sur le serveur! Le rôle Membre t'a été attribué`,
            ephemeral: true,
        });
    },
};