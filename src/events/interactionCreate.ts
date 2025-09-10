import {
    Events,
    type Interaction,
    GuildMember,
    ButtonBuilder,
    ActionRowBuilder,
    ButtonStyle,
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
                content: `❌ This button isn't for you!`,
                ephemeral: true,
            });
        }

        const member = interaction.member as GuildMember;
        const roleId = process.env.MEMBER_ROLE_ID!;
        const role = interaction.guild?.roles.cache.get(roleId);

        if (!role) {
            return interaction.reply({
                content: `⚠️ "Member" role cannot be found.`,
                ephemeral: true,
            });
        }

        if (member.roles.cache.has(role.id)) {
            return interaction.reply({
                content: `✅ You're already a member!`,
                ephemeral: true,
            });
        }

        await member.roles.add(role);

        const disabledButton = new ButtonBuilder()
            .setCustomId(`welcome-role-${member.id}`)
            .setLabel('✅ Joined!')
            .setStyle(ButtonStyle.Success)
            .setDisabled(true);
        const row = new ActionRowBuilder<ButtonBuilder>().addComponents(disabledButton);

        await interaction.update({
            content: interaction.message.content,
            embeds: interaction.message.embeds,
            components: [row],
        });

        await interaction.followUp({
            content: `🎉 Welcome to Le Socle! "Member" role has been assigned to you.`,
            ephemeral: true,
        });
    },
};
